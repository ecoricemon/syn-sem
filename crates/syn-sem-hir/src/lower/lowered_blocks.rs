use crate::{BlockId, ExprId, Hir, ItemId, LocalId, PatId};
use std::ops::Index;
use syn_sem_name::DefId;

/// Lowered views for all HIR source blocks, indexed by [`BlockId`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoweredBlocks {
    blocks: Vec<Block>,
}

impl LoweredBlocks {
    pub(crate) fn from_hir(hir: &Hir<'_>) -> Self {
        let blocks = hir
            .blocks()
            .iter()
            .map(|block| Block::from_hir_block(hir, block.id))
            .collect();
        Self { blocks }
    }

    /// Returns all lowered blocks.
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }
}

impl Index<BlockId> for LoweredBlocks {
    type Output = Block;

    fn index(&self, id: BlockId) -> &Self::Output {
        &self.blocks[id.index()]
    }
}

/// Lowered view for one HIR source block.
///
/// Unlike [`crate::Block`], this does not retain the original AST block or scope linkage.
/// It keeps the statement shape and tail expression that upper semantic phases need after the
/// source-spine block has been built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// Source block id.
    pub block: BlockId,
    /// Lowered statements in source order.
    pub stmts: Vec<Stmt>,
    /// Tail expression for this block, when the final expression has no trailing semicolon.
    pub tail_expr: Option<ExprId>,
}

impl Block {
    fn from_hir_block(hir: &Hir<'_>, block: BlockId) -> Self {
        let stmts = hir[block]
            .stmts
            .iter()
            .map(|stmt| Stmt::from_hir_stmt(hir, *stmt))
            .collect();
        let tail_expr = hir[block]
            .stmts
            .last()
            .and_then(|stmt| match hir[*stmt].kind {
                crate::StmtKind::Expr {
                    expr,
                    has_semi: false,
                } => Some(expr),
                _ => None,
            });
        Self {
            block,
            stmts,
            tail_expr,
        }
    }
}

/// Lowered view of one HIR source statement.
///
/// Unlike [`crate::Stmt`], this has no AST statement reference or statement-local scope. It keeps
/// only the statement data that later phases consume from block lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    /// Local binding statement.
    Local(Local),
    /// Block-local item statement.
    Item(ItemId),
    /// Expression statement.
    Expr(ExprId),
}

impl Stmt {
    fn from_hir_stmt(hir: &Hir<'_>, stmt: crate::StmtId) -> Self {
        match hir[stmt].kind {
            crate::StmtKind::Local(local) => {
                let pat = hir[local].pat;
                Self::Local(Local {
                    local,
                    pat,
                    bindings: collect_pat_bindings(hir, pat),
                    init: hir[local].init,
                })
            }
            crate::StmtKind::Item(item) => Self::Item(item),
            crate::StmtKind::Expr { expr, .. } => Self::Expr(expr),
        }
    }
}

/// Lowered view of one HIR source local binding.
///
/// Unlike [`crate::Local`], this expands the binding pattern into the local definitions it
/// introduces, so upper phases can consume the binding facts without re-walking the pattern spine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local {
    /// Source local id.
    pub local: LocalId,
    /// Source pattern for this local binding.
    pub pat: PatId,
    /// Local definitions introduced by this local's binding pattern, in source order.
    pub bindings: Vec<DefId>,
    /// Optional initializer expression.
    pub init: Option<ExprId>,
}

fn collect_pat_bindings(hir: &Hir<'_>, pat: PatId) -> Vec<DefId> {
    let mut bindings = Vec::new();
    collect_pat_bindings_into(hir, pat, &mut bindings);
    bindings
}

fn collect_pat_bindings_into(hir: &Hir<'_>, pat: PatId, bindings: &mut Vec<DefId>) {
    match &hir[pat].kind {
        crate::PatKind::Ident { def, .. } => {
            if let Some(def) = def {
                bindings.push(*def);
            }
        }
        crate::PatKind::Reference { pat, .. } | crate::PatKind::Type { pat, .. } => {
            collect_pat_bindings_into(hir, *pat, bindings);
        }
        crate::PatKind::Struct { fields, .. } => {
            for field in fields {
                collect_pat_bindings_into(hir, field.pat, bindings);
            }
        }
        crate::PatKind::Tuple { elems } => {
            for elem in elems {
                collect_pat_bindings_into(hir, *elem, bindings);
            }
        }
        crate::PatKind::Path(_) | crate::PatKind::Unsupported => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HirBuilder, ItemKind};
    use syn_sem_ast as ast;
    use syn_sem_ast::SyntaxCx;
    use syn_sem_common::CommonCx;
    use syn_sem_name::{collect::NameCollector, DefKind};

    fn parsed_hir<'cx>(
        ccx: &'cx CommonCx,
        scx: &'cx SyntaxCx<'cx>,
        source_text: &str,
    ) -> (syn_sem_name::NameDb<'cx>, crate::Hir<'cx>) {
        let file_path = ccx.intern("body_lower_test.rs");
        let source_text = ccx.intern(source_text);
        scx.parse_virtual_file(file_path, source_text)
            .expect("test input should parse");
        let file = scx.lookup_source(file_path).unwrap().ast();
        let names = NameCollector::collect([ast::SourceInput { file_path, file }], [file_path])
            .expect("name collection should succeed");
        let hir = HirBuilder::new(&names).build(file_path, file);
        (names, hir)
    }

    fn function_block(hir: &crate::Hir<'_>, name: &str) -> BlockId {
        let item = hir
            .items()
            .iter()
            .find(|item| {
                item.name
                    .is_some_and(|item_name| item_name.as_ref() == name)
            })
            .unwrap_or_else(|| panic!("expected function `{name}`"));
        let ItemKind::Fn { block, .. } = item.kind else {
            panic!("expected function item");
        };
        block
    }

    #[test]
    fn lowers_block_statement_order_and_local_bindings() {
        // Proves lowered blocks preserve statement order and local binding defs.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (names, hir) = parsed_hir(
            &ccx,
            &scx,
            r#"
            fn f(pair: (usize, usize)) {
                let (a, ref mut b) = pair;
                struct LocalItem;
                a
            }
            "#,
        );

        let block = &hir.lowered_blocks()[function_block(&hir, "f")];

        let [Stmt::Local(local), Stmt::Item(item), Stmt::Expr(_)] = block.stmts.as_slice() else {
            panic!("expected local, item, expr statement order");
        };
        assert!(block.tail_expr.is_some());
        assert_eq!(local.pat, hir[local.local].pat);
        assert_eq!(local.bindings.len(), 2);
        assert!(local.init.is_some());
        assert!(local
            .bindings
            .iter()
            .all(|def| names[*def].kind == DefKind::Local));
        assert_eq!(names[local.bindings[0]].name.unwrap().as_ref(), "a");
        assert_eq!(names[local.bindings[1]].name.unwrap().as_ref(), "b");
        assert_eq!(hir[*item].name.unwrap().as_ref(), "LocalItem");
    }

    #[test]
    fn lowers_tail_expression_only_without_semicolon() {
        // Proves only expression statements without semicolons become tail expressions.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_, hir) = parsed_hir(
            &ccx,
            &scx,
            r#"
            fn with_tail(x: usize) -> usize { x }
            fn without_tail(x: usize) { x; }
            "#,
        );

        let with_tail = &hir.lowered_blocks()[function_block(&hir, "with_tail")];
        let without_tail = &hir.lowered_blocks()[function_block(&hir, "without_tail")];

        assert!(with_tail.tail_expr.is_some());
        assert_eq!(without_tail.tail_expr, None);
    }
}
