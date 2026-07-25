//! Dependency planning for compile-time constant evaluation.

use crate::required::required_exprs;
use std::{collections::VecDeque, hash::Hash};
use syn_sem_common::{GraphNodeId, InternedDirectedGraph, Map, MaybeResult, Result, Set};
use syn_sem_hir as hir;
use syn_sem_name::{DefId, DefKind, NameDb, ResolveResult};

/// One value-producing node in a constant-evaluation dependency graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum EvalNode {
    /// A HIR expression.
    Expr(hir::ExprId),

    /// A constant item definition, identified independently of its path occurrences.
    ///
    /// For `const A: usize = 1; const B: usize = A + 1;`, this node uses the `DefId` created for
    /// the declaration of `A`. The `A` inside `B` is a separate [`Self::Expr`] path node that
    /// resolves to the same definition, producing an edge from `Const(A)` to `Expr(A)`.
    Const(DefId),
}

/// A reusable dependency plan for constant evaluation.
///
/// Edges point from a dependency to its dependent. The plan depends only on HIR and name facts, so
/// callers can build its dependency-first order once and reuse it across inference/evaluation
/// fixed-point iterations. Nodes in or downstream of a cycle are kept out of that order, so their
/// values remain unavailable. The dependency graph and schedule are internal details; callers pass
/// the completed plan to [`crate::EvalDb::analyze`].
#[derive(Debug)]
pub struct EvalPlan {
    /// Nodes and dependency edges needed to evaluate the selected targets.
    ///
    /// Every edge points from a dependency to its dependent. For example, given
    /// `const A: usize = a + b;`, where `a` and `b` resolve to const items, the relevant part of
    /// the graph has this shape:
    ///
    /// ```text
    /// Expr(a) and Expr(b) -> Expr(a + b) -> Const(A)
    /// ```
    ///
    /// The two path occurrences are distinct expression nodes. The binary expression depends on
    /// both operands, and the definition of `A` depends on its initializer expression.
    graph: InternedDirectedGraph<EvalNode>,

    /// Graph nodes whose values are requested directly by constant evaluation.
    ///
    /// In `const A: usize = a + b;`, `Const(A)` is a target, while the operand paths and the binary
    /// initializer are its dependencies. If the same constant is used as an array length in
    /// `type Array = [u8; A];`, that particular `Expr(A)` occurrence is also a target.
    ///
    /// Targets describe requested results rather than a structural property of the graph. A target
    /// may have both incoming and outgoing edges. Const items referenced by `A` may also appear as
    /// targets because every const item definition is evaluated directly.
    targets: Vec<GraphNodeId>,

    /// Reachable nodes in dependency-first evaluation order.
    order: Vec<GraphNodeId>,

    const_item_inits: Map<DefId, hir::ExprId>,
    const_item_types: Map<DefId, hir::TypeId>,
}

impl EvalPlan {
    /// Builds a constant-evaluation dependency plan from HIR and name facts.
    pub fn new<'cx>(hir: &hir::Hir<'cx>, names: &NameDb<'cx>) -> Result<Self> {
        EvalPlanBuilder {
            hir,
            names,
            plan: Self {
                graph: InternedDirectedGraph::new(),
                targets: Vec::new(),
                order: Vec::new(),
                const_item_inits: Map::default(),
                const_item_types: Map::default(),
            },
            expanded_exprs: Set::default(),
            expanded_consts: Set::default(),
            target_nodes: Set::default(),
        }
        .build()
    }

    /// Returns the graph described by [`Self::graph`].
    pub(crate) fn graph(&self) -> &InternedDirectedGraph<EvalNode> {
        &self.graph
    }

    pub(crate) fn order(&self) -> &[GraphNodeId] {
        &self.order
    }

    pub(crate) fn const_item_init(&self, def: DefId) -> Option<hir::ExprId> {
        self.const_item_inits.get(&def).copied()
    }

    pub(crate) fn const_item_type(&self, def: DefId) -> Option<hir::TypeId> {
        self.const_item_types.get(&def).copied()
    }
}

struct EvalPlanBuilder<'hir, 'cx> {
    hir: &'hir hir::Hir<'cx>,
    names: &'hir NameDb<'cx>,
    plan: EvalPlan,
    expanded_exprs: Set<hir::ExprId>,
    expanded_consts: Set<DefId>,
    target_nodes: Set<GraphNodeId>,
}

impl EvalPlanBuilder<'_, '_> {
    fn build(mut self) -> Result<EvalPlan> {
        let const_defs = self.collect_const_items();
        for def in const_defs {
            let target = self.collect_const(def)?;
            self.insert_target(target);
        }
        for expr in required_exprs(self.hir) {
            let target = self.collect_expr(expr)?;
            self.insert_target(target);
        }
        self.finish_schedule();
        Ok(self.plan)
    }

    fn finish_schedule(&mut self) {
        let (order, _) = dependency_schedule(&self.plan.graph, &self.plan.targets);
        self.plan.order = order;
    }

    fn collect_const_items(&mut self) -> Vec<DefId> {
        let mut defs = Vec::new();
        for item in self.hir.items() {
            let hir::ItemKind::Const { ty, init } = item.kind else {
                continue;
            };
            let Some(def) = item.def else {
                continue;
            };
            self.plan.const_item_inits.insert(def, init);
            self.plan.const_item_types.insert(def, ty);
            defs.push(def);
        }
        defs
    }

    fn collect_const(&mut self, def: DefId) -> Result<GraphNodeId> {
        let node = self.plan.graph.intern(EvalNode::Const(def));
        if !self.expanded_consts.insert(def) {
            return Ok(node);
        }
        if let Some(init) = self.plan.const_item_inits.get(&def).copied() {
            let dependency = self.collect_expr(init)?;
            self.plan.graph.add_edge(dependency, node);
        }
        Ok(node)
    }

    fn collect_expr(&mut self, expr: hir::ExprId) -> Result<GraphNodeId> {
        let node = self.plan.graph.intern(EvalNode::Expr(expr));
        if !self.expanded_exprs.insert(expr) {
            return Ok(node);
        }

        match &self.hir[expr].kind {
            hir::ExprKind::Binary { left, right, .. } => {
                self.add_expr_dependency(*left, node)?;
                self.add_expr_dependency(*right, node)?;
            }
            hir::ExprKind::Block { block } | hir::ExprKind::Const { block } => {
                if let Some(tail) = self.hir.lowered_blocks()[*block].tail_expr {
                    self.add_expr_dependency(tail, node)?;
                }
            }
            hir::ExprKind::Cast {
                expr: dependency, ..
            }
            | hir::ExprKind::Paren { expr: dependency }
            | hir::ExprKind::Unary {
                expr: dependency, ..
            } => self.add_expr_dependency(*dependency, node)?,
            hir::ExprKind::Path(path) => {
                if let Some(def) = resolve_const_item(self.names, path, self.hir[expr].scope)? {
                    let dependency = self.collect_const(def)?;
                    self.plan.graph.add_edge(dependency, node);
                }
            }
            hir::ExprKind::Lit(_)
            | hir::ExprKind::Array { .. }
            | hir::ExprKind::Assign { .. }
            | hir::ExprKind::Call { .. }
            | hir::ExprKind::Closure { .. }
            | hir::ExprKind::Field { .. }
            | hir::ExprKind::Index { .. }
            | hir::ExprKind::MethodCall { .. }
            | hir::ExprKind::Reference { .. }
            | hir::ExprKind::Repeat { .. }
            | hir::ExprKind::Return { .. }
            | hir::ExprKind::Struct { .. }
            | hir::ExprKind::Tuple { .. } => {}
        }
        Ok(node)
    }

    fn add_expr_dependency(
        &mut self,
        dependency: hir::ExprId,
        dependent: GraphNodeId,
    ) -> Result<()> {
        let dependency = self.collect_expr(dependency)?;
        self.plan.graph.add_edge(dependency, dependent);
        Ok(())
    }

    fn insert_target(&mut self, target: GraphNodeId) {
        if self.target_nodes.insert(target) {
            self.plan.targets.push(target);
        }
    }
}

fn dependency_schedule<N>(
    graph: &InternedDirectedGraph<N>,
    targets: &[GraphNodeId],
) -> (Vec<GraphNodeId>, Vec<GraphNodeId>)
where
    N: Eq + Hash,
{
    let mut reachable = Set::default();
    let mut pending = targets.to_vec();
    while let Some(node) = pending.pop() {
        if !reachable.insert(node) {
            continue;
        }
        pending.extend(graph.incoming(node).iter().copied());
    }

    let mut dependency_counts = vec![0usize; graph.len()];
    for node in graph.node_ids() {
        if reachable.contains(&node) {
            dependency_counts[node.index()] = graph
                .incoming(node)
                .iter()
                .filter(|dependency| reachable.contains(dependency))
                .count();
        }
    }

    let mut ready = graph
        .node_ids()
        .filter(|node| reachable.contains(node) && dependency_counts[node.index()] == 0)
        .collect::<VecDeque<_>>();
    let mut order = Vec::with_capacity(reachable.len());
    while let Some(node) = ready.pop_front() {
        order.push(node);
        for dependent in graph.outgoing(node) {
            if !reachable.contains(dependent) {
                continue;
            }
            let count = &mut dependency_counts[dependent.index()];
            *count = count
                .checked_sub(1)
                .expect("scheduled dependency counts must stay balanced");
            if *count == 0 {
                ready.push_back(*dependent);
            }
        }
    }

    let ordered = order.iter().copied().collect::<Set<_>>();
    let blocked = graph
        .node_ids()
        .filter(|node| reachable.contains(node) && !ordered.contains(node))
        .collect();
    (order, blocked)
}

pub(crate) fn resolve_const_item<'cx>(
    names: &NameDb<'cx>,
    path: &hir::Path<'cx>,
    scope: Option<syn_sem_name::ScopeId>,
) -> MaybeResult<DefId> {
    if path.qself.is_some() || path.segments.iter().any(|segment| !segment.args.is_empty()) {
        return Err("constant evaluation: unsupported const path shape".into());
    }
    let Some(scope) = scope else {
        return Ok(None);
    };
    let ResolveResult::Found(def) =
        names.resolve_value_path(scope, path.segments.iter().map(|segment| segment.name))
    else {
        return Ok(None);
    };
    Ok((names[def].kind == DefKind::Const).then_some(def))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedules_shared_dependencies_once_before_dependents() {
        let mut graph = InternedDirectedGraph::new();
        let dependency = graph.intern("dependency");
        let shared = graph.intern("shared");
        let left = graph.intern("left");
        let right = graph.intern("right");
        let orphan = graph.intern("orphan");
        graph.add_edge(dependency, shared);
        graph.add_edge(shared, left);
        graph.add_edge(shared, right);

        let (order, blocked) = dependency_schedule(&graph, &[left, right]);
        let position = |node| {
            order
                .iter()
                .position(|ordered| *ordered == node)
                .expect("reachable acyclic node should be ordered")
        };

        assert!(position(dependency) < position(shared));
        assert!(position(shared) < position(left));
        assert!(position(shared) < position(right));
        assert_eq!(order.iter().copied().collect::<Set<_>>().len(), order.len());
        assert!(!order.contains(&orphan));
        assert!(blocked.is_empty());
    }

    #[test]
    fn blocks_cycles_and_their_dependents_without_blocking_independent_nodes() {
        let mut graph = InternedDirectedGraph::new();
        let cycle_left = graph.intern("cycle_left");
        let cycle_right = graph.intern("cycle_right");
        let downstream = graph.intern("downstream");
        let independent_dependency = graph.intern("independent_dependency");
        let independent = graph.intern("independent");
        graph.add_edge(cycle_left, cycle_right);
        graph.add_edge(cycle_right, cycle_left);
        graph.add_edge(cycle_right, downstream);
        graph.add_edge(independent_dependency, independent);

        let (order, blocked) = dependency_schedule(&graph, &[downstream, independent]);

        assert_eq!(blocked, vec![cycle_left, cycle_right, downstream]);
        assert_eq!(order, vec![independent_dependency, independent]);
    }
}
