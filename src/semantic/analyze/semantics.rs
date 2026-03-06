use super::{
    find_known::KnownLibFinder,
    handler,
    infer_eval::Inspector,
    resolve::shorten_chain,
    task::{GenerateTask, Task, TaskId, TaskItem, TaskQueue, TaskResolveUse},
};
use crate::{
    ds::vec::BoxedSlice,
    etc::{abs_fs::AbstractFiles, known, util::FiniteLoop},
    semantic::{
        entry::{context::ConfigLoad, GlobalCx},
        eval::Evaluated,
        logic::Logic,
        tree::{PathId, PrivItem, PrivPathTree, PubPathTree, SynToPath, Type, TypeId, TypeScalar},
    },
    syntax::{common::SynId, SyntaxTree},
    Result, TriError, TriResult, TriResultHelper,
};
use std::{
    iter,
    path::{Path as StdPath, PathBuf},
    sync::Arc,
};

pub struct Analyzer<'gcx> {
    gcx: &'gcx GlobalCx<'gcx>,
    files: AbstractFiles,
    stree: SyntaxTree,
    ptree: PubPathTree<'gcx>,
    s2p: SynToPath,
    evaluated: Evaluated<'gcx>,
    logic: Logic<'gcx>,
    type_inspector: Inspector<'gcx>,
    known_finder: KnownLibFinder,
    tasks: TaskQueue<'gcx>,
}

impl<'gcx> Analyzer<'gcx> {
    pub fn new(gcx: &'gcx GlobalCx<'gcx>) -> Self {
        Self {
            gcx,
            files: AbstractFiles::default(),
            stree: SyntaxTree::new(),
            ptree: PubPathTree::new(PrivPathTree::new()),
            s2p: SynToPath::new(),
            evaluated: Evaluated::new(),
            logic: Logic::new(gcx),
            type_inspector: Inspector::new(gcx),
            known_finder: KnownLibFinder::new(),
            tasks: TaskQueue::new(),
        }
    }

    pub fn get_global_context(&self) -> &GlobalCx<'gcx> {
        self.gcx
    }

    pub fn add_virtual_file<P, C>(&mut self, path: P, code: C) -> Option<Arc<str>>
    where
        P: Into<PathBuf>,
        C: Into<Arc<str>>,
    {
        self.files.insert_virtual_file(path.into(), code.into())
    }

    pub fn set_known_library<N, P>(&mut self, name: N, path: P) -> Result<()>
    where
        N: Into<Box<str>>,
        P: AsRef<StdPath>,
    {
        self.files
            .set_known_library(name.into(), path.as_ref())
            .map(|_| ())
    }

    pub fn analyze(self, entry: impl AsRef<StdPath>) -> Result<Semantics<'gcx>> {
        fn inner<'gcx>(this: Analyzer<'gcx>, entry: &StdPath) -> Result<Semantics<'gcx>> {
            let fpath = entry.to_path_buf();
            let npath = this.files.to_name_path(entry)?;
            let entry_task = Task::construct_path_tree_for_file(fpath, npath);

            let mut sem = Semantics {
                gcx: this.gcx,
                files: this.files,
                stree: this.stree,
                ptree: this.ptree,
                s2p: this.s2p,
                evaluated: this.evaluated,
                logic: this.logic,
                type_inspector: this.type_inspector,
                known_finder: this.known_finder,
                tasks: this.tasks,
            };

            let mut cx = sem.as_analyze_cx();
            cx.add_default()?;
            cx.run_task_loop_with_task(entry_task)?;
            Ok(sem)
        }
        inner(self, entry.as_ref())
    }
}

#[derive(Debug)]
pub struct Semantics<'gcx> {
    gcx: &'gcx GlobalCx<'gcx>,
    pub files: AbstractFiles,
    pub stree: SyntaxTree,
    pub ptree: PubPathTree<'gcx>,
    pub s2p: SynToPath,
    pub evaluated: Evaluated<'gcx>,
    pub logic: Logic<'gcx>,
    type_inspector: Inspector<'gcx>,
    known_finder: KnownLibFinder,
    tasks: TaskQueue<'gcx>,
}

impl<'gcx> Semantics<'gcx> {
    pub fn monomorphize_impl(&mut self, item_impl: SynId, self_ty: Option<TypeId>) -> Result<()> {
        let task = Task::monomorphize_impl(item_impl, self_ty);
        self.as_analyze_cx().run_task_loop_with_task(task)
    }

    pub fn get_type_of_expr(&mut self, expr: SynId) -> Option<TypeId> {
        // Gets infer type if the expression has been inferred.
        let syn = expr.as_any().downcast_ref::<syn::Expr>()?;
        let infer_ty = self.type_inspector.inferer.get_type(syn)?.clone();

        let base = crate::helper::ptree::find_base_node_of_expr(&self.stree, &self.s2p, expr)?;

        let mut dummy_tasks = TaskQueue::new();
        let infer_helper = self.type_inspector.as_infer_helper(
            self.gcx,
            &self.stree,
            &self.ptree.inner,
            &self.s2p,
            &self.evaluated,
            &mut self.logic,
            &mut dummy_tasks,
            base,
        );

        TypeId::from_infer_type(infer_ty, &infer_helper).ok()
    }

    fn as_analyze_cx(&mut self) -> AnalyzeCx<'_, 'gcx> {
        AnalyzeCx {
            gcx: self.gcx,
            files: &mut self.files,
            stree: &mut self.stree,
            ptree: &mut self.ptree.inner,
            s2p: &mut self.s2p,
            evaluated: &mut self.evaluated,
            type_inspector: &mut self.type_inspector,
            logic: &mut self.logic,
            known_finder: &mut self.known_finder,
            tasks: &mut self.tasks,
        }
    }
}

struct AnalyzeCx<'a, 'gcx> {
    gcx: &'gcx GlobalCx<'gcx>,
    files: &'a mut AbstractFiles,
    stree: &'a mut SyntaxTree,
    ptree: &'a mut PrivPathTree<'gcx>,
    s2p: &'a mut SynToPath,
    evaluated: &'a mut Evaluated<'gcx>,
    logic: &'a mut Logic<'gcx>,
    type_inspector: &'a mut Inspector<'gcx>,
    known_finder: &'a mut KnownLibFinder,
    tasks: &'a mut TaskQueue<'gcx>,
}

impl<'gcx> AnalyzeCx<'_, 'gcx> {
    fn run_task_loop_with_task(&mut self, task: Task<'gcx>) -> Result<()> {
        let _ = self.tasks.push_back(task);
        self.run_task_loop()
    }

    fn run_task_loop(&mut self) -> Result<()> {
        const LOOP_ID: &str = "analyze-loop";
        FiniteLoop::set_limit(LOOP_ID, 10);
        FiniteLoop::reset(LOOP_ID);

        let mut cx_tasks = Vec::new();

        // Main task handling loop
        while let Some(task_item) = self.tasks.pop_front() {
            // Panics if infinite loop detected
            let key = iter::once(TaskId::from(&task_item.task))
                .chain(self.tasks.iter().map(TaskId::from));
            FiniteLoop::assert(LOOP_ID, key, || {
                let tasks = iter::once(&task_item.task)
                    .chain(&*self.tasks)
                    .collect::<BoxedSlice<_>>();
                panic!("infinite loop detected: remaining tasks\n{:?}", tasks);
            });

            let TaskItem {
                task,
                this_node,
                parent_node,
            } = task_item;

            // Task is going to be processed in this order.
            // 1. Setup - Some tasks need some setup procedures. We search all setup tasks from
            //    root node to current node(task), then process them in the order.
            // 2. Process - Processes this task.
            // 3. Cleanup - Some tasks need some cleanup procedures. We search all cleanup tasks
            //    from current node(task) to root node, then process them in the order.

            // 1. Sets up
            self.tasks.append_setup_tasks(this_node, &mut cx_tasks);
            for setup_task in cx_tasks.drain(..) {
                self.call_task_handler(setup_task).unwrap(); // never fails
            }

            // 2. Processes the task.
            let had_cleanup = self.tasks.get_cleanup_task(this_node).is_some();
            let res = self.handle_task(task);

            // 3. Cleans up
            if !had_cleanup && self.tasks.get_cleanup_task(this_node).is_some() {
                // If current task just made cleanup task, it is for children tasks, not for the
                // current task. So we skip it.
                self.tasks.append_cleanup_tasks(parent_node, &mut cx_tasks);
            } else {
                self.tasks.append_cleanup_tasks(this_node, &mut cx_tasks);
            }
            for cleanup_task in cx_tasks.drain(..) {
                self.call_task_handler(cleanup_task).unwrap(); // never fails
            }

            match res {
                Ok(()) => self.tasks.mark_done(this_node),
                Err(e) => match e {
                    TriError::Soft(task) => {
                        // Retries the task again
                        self.tasks.push_back_force(TaskItem {
                            task,
                            this_node,
                            parent_node,
                        });
                    }
                    TriError::Hard(e) => return Err(e),
                },
            }
        }

        shorten_chain(self.ptree);

        debug_assert_eq!(self.ptree.unresolved().len(), 0);
        debug_assert!(self.tasks.is_empty());
        self.tasks.reset();
        Ok(())
    }

    fn handle_task(&mut self, task: Task<'gcx>) -> TriResult<(), Task<'gcx>> {
        let res = self.call_task_handler(task);

        fn append_task<'gcx, T: GenerateTask<'gcx>>(
            t: &T,
            pid: PathId,
            tasks: &mut TaskQueue<'gcx>,
        ) {
            for task in t.generate_task(pid) {
                let _ = tasks.push_back(task);
            }
        }

        let mut found_raw_use = false;

        for pid in self.ptree.unresolved() {
            match &self.ptree[pid] {
                PrivItem::RawConst(raw) => append_task(raw, pid, self.tasks),
                PrivItem::RawEnum(raw) => append_task(raw, pid, self.tasks),
                PrivItem::RawField(raw) => append_task(raw, pid, self.tasks),
                PrivItem::RawFn(raw) => append_task(raw, pid, self.tasks),
                PrivItem::RawLocal(raw) => append_task(raw, pid, self.tasks),
                PrivItem::RawMod(raw) => append_task(raw, pid, self.tasks),
                PrivItem::RawStruct(raw) => append_task(raw, pid, self.tasks),
                PrivItem::RawTrait(raw) => append_task(raw, pid, self.tasks),
                PrivItem::RawTypeAlias(raw) => append_task(raw, pid, self.tasks),
                PrivItem::RawUse(raw) => {
                    append_task(raw, pid, self.tasks);
                    found_raw_use = true;
                }
                PrivItem::RawVariant(raw) => append_task(raw, pid, self.tasks),

                PrivItem::Block(_)
                | PrivItem::Const(_)
                | PrivItem::Enum(_)
                | PrivItem::Field(_)
                | PrivItem::Fn(_)
                | PrivItem::Local(_)
                | PrivItem::Mod(_)
                | PrivItem::Struct(_)
                | PrivItem::Trait(_)
                | PrivItem::TypeAlias(_)
                | PrivItem::Use(_)
                | PrivItem::Variant(_)
                | PrivItem::None => { /* No need to resolve further */ }
            }
        }

        if found_raw_use {
            for task in TaskResolveUse::tasks_for_all() {
                let _ = self.tasks.push_back(task);
            }
        }

        res
    }

    fn call_task_handler(&mut self, task: Task<'gcx>) -> TriResult<(), Task<'gcx>> {
        match task {
            Task::ConstructPathTree(inner) => handler::TaskConstructPathTreeHandler {
                files: self.files,
                stree: self.stree,
                ptree: self.ptree,
                s2p: self.s2p,
                tasks: self.tasks,
            }
            .handle_task(inner)
            .map_soft_err(Task::ConstructPathTree),
            Task::FindKnownLib(inner) => {
                handler::TaskFindKnownLibHandler {
                    files: self.files,
                    known_finder: self.known_finder,
                    tasks: self.tasks,
                }
                .handle_task(inner);
                Ok(())
            }
            Task::LoadLogic(inner) => handler::TaskLoadLogicHandler {
                gcx: self.gcx,
                ptree: self.ptree,
                s2p: self.s2p,
                evaluated: self.evaluated,
                type_inspector: self.type_inspector,
                logic: self.logic,
                tasks: self.tasks,
            }
            .handle_task(inner)
            .map_soft_err(Task::LoadLogic),
            Task::Resolve(inner) => handler::TaskResolveHandler {
                gcx: self.gcx,
                stree: self.stree,
                ptree: self.ptree,
                s2p: self.s2p,
                evaluated: self.evaluated,
                type_inspector: self.type_inspector,
                logic: self.logic,
                tasks: self.tasks,
            }
            .handle_task(inner)
            .map_soft_err(Task::Resolve),
            Task::FixType(inner) => handler::TaskFixTypeHandler {
                gcx: self.gcx,
                stree: self.stree,
                ptree: self.ptree,
                s2p: self.s2p,
                evaluted: self.evaluated,
                type_inspector: self.type_inspector,
                logic: self.logic,
                tasks: self.tasks,
            }
            .handle_task(inner)
            .map_soft_err(Task::FixType),
            Task::EvalConst(inner) => handler::TaskEvalConstHandler {
                gcx: self.gcx,
                stree: self.stree,
                ptree: self.ptree,
                s2p: self.s2p,
                evaluated: self.evaluated,
                type_inspector: self.type_inspector,
                logic: self.logic,
                tasks: self.tasks,
            }
            .handle_task(inner)
            .map_soft_err(Task::EvalConst),
            Task::EvalExpr(inner) => handler::TaskEvalExprHandler {
                gcx: self.gcx,
                stree: self.stree,
                ptree: self.ptree,
                s2p: self.s2p,
                evaluated: self.evaluated,
                type_inspector: self.type_inspector,
                logic: self.logic,
                tasks: self.tasks,
            }
            .handle_task(inner)
            .map_soft_err(Task::EvalExpr),
            Task::Monomorphize(inner) => handler::TaskMonomorphizeHandler {
                gcx: self.gcx,
                stree: self.stree,
                ptree: self.ptree,
                s2p: self.s2p,
                inferer: &mut self.type_inspector.inferer,
                tasks: self.tasks,
            }
            .handle_task(inner)
            .map_soft_err(Task::Monomorphize),
            Task::Dyn(inner) => handler::TaskDynHandler { gcx: self.gcx }
                .handle_task(inner)
                .map_soft_err(Task::Dyn),
        }
    }

    fn add_default(&mut self) -> Result<()> {
        // Adds default scalar types such as "i32" and "u32".
        for name in known::scalar_names() {
            let scalar = TypeScalar::from_type_name(name).unwrap();
            self.ptree.insert_type(Type::Scalar(scalar));
        }

        // Appends a task for reading default well known libraries such as "core" and "std" if
        // the configuration allows.
        let config = self.gcx.get_config();
        for (name, code, flag) in [
            ("core", known::LIB_CORE_CODE, ConfigLoad::CORE),
            ("std", known::LIB_STD_CODE, ConfigLoad::STD),
        ] {
            if !config.load.contains(flag) {
                continue;
            }
            // Registers the well known library code.
            let fpath: PathBuf = name.into();
            let code: Arc<str> = code.into();
            self.files.insert_virtual_file(fpath.clone(), code);
            self.files.set_known_library(name.into(), name.as_ref())?;

            // Adds a task for the library file.
            let npath: String = name.to_owned();
            let task = Task::construct_path_tree_for_file(fpath, npath);
            let _ = self.tasks.push_back(task);
        }
        self.run_task_loop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        etc::util,
        pid, pitem, pnode,
        semantic::tree::{ArrayLen, OwnedParam, OwnedType, PathId, PubItem},
        Config, ConfigLoad, GetOwned,
    };

    fn prepare<'gcx, 'a>(
        gcx: &'gcx GlobalCx<'gcx>,
        files: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Analyzer<'gcx> {
        syn_locator::enable_thread_local(true);
        syn_locator::clear();

        util::set_crate_name(util::cargo_crate_name());

        let mut analyzer = Analyzer::new(gcx);
        for (path, code) in files {
            analyzer.add_virtual_file(path, code);
        }
        analyzer
    }

    mod import {
        use super::*;
        use crate::semantic::tree::NodeIndex;

        #[test]
        fn test_cascading_import() {
            let entry = "/mod.rs";
            let code = r#"
            mod a {
                pub mod b {
                    pub struct C;
                }
            }
            use b::C;
            use a::b;
            "#;

            let gcx = GlobalCx::default();
            gcx.configure(Config {
                load: ConfigLoad::empty(),
            });
            let sem = prepare(&gcx, [(entry, code)]).analyze(entry).unwrap();
            let ptree = &sem.ptree;
            let crate_ = ptree.crate_name();

            let b = pitem!(ptree, "{crate_}::mod::b");
            assert_eq!(b.as_use().unwrap().dst, pid!(ptree, "{crate_}::mod::a::b"));

            let c = pitem!(ptree, "{crate_}::mod::C");
            assert_eq!(
                c.as_use().unwrap().dst,
                pid!(ptree, "{crate_}::mod::a::b::C")
            );
        }

        #[test]
        fn test_recursive_import() {
            let entry = "/mod.rs";
            let code = r#"
            mod a {
                pub struct A;
                struct AA;
                pub use super::b::*;
            }
            mod b {
                pub struct B;
                struct BB;
                pub use super::c::*;
            }
            mod c {
                pub struct C;
                struct CC;
                pub use super::a::*;
            }
            "#;

            let gcx = GlobalCx::default();
            gcx.configure(Config {
                load: ConfigLoad::empty(),
            });
            let sem = prepare(&gcx, [(entry, code)]).analyze(entry).unwrap();
            let ptree = &sem.ptree;
            let crate_ = ptree.crate_name();

            let node_a = pnode!(ptree, "{crate_}::mod::a::A");
            let node_b = pnode!(ptree, "{crate_}::mod::b::B");
            let node_c = pnode!(ptree, "{crate_}::mod::c::C");

            assert_eq!(ptree.node(node_a).iter().count(), 1);
            assert_eq!(ptree.node(node_b).iter().count(), 1);
            assert_eq!(ptree.node(node_c).iter().count(), 1);

            let pid_a = node_a.to_path_id(0);
            let pid_b = node_b.to_path_id(0);
            let pid_c = node_c.to_path_id(0);

            let test = |key: &str, expected_dst: PathId| {
                let node = pnode!(ptree, "{crate_}::{key}");
                let node = ptree.node(node);
                let mut values = node.iter();

                if values.clone().count() != 1 {
                    panic!("error at key: `{key}`");
                }

                let (_, value) = values.next().unwrap();
                assert_eq!(value.as_use().unwrap().dst, expected_dst);
            };

            test("mod::a::B", pid_b);
            test("mod::a::C", pid_c);
            test("mod::b::A", pid_a);
            test("mod::b::C", pid_c);
            test("mod::c::A", pid_a);
            test("mod::c::B", pid_b);

            let root = PubPathTree::ROOT;
            assert!(ptree.search(root, "crate::mod::a::BB").is_none());
            assert!(ptree.search(root, "crate::mod::a::CC").is_none());
            assert!(ptree.search(root, "crate::mod::b::AA").is_none());
            assert!(ptree.search(root, "crate::mod::b::CC").is_none());
            assert!(ptree.search(root, "crate::mod::c::AA").is_none());
            assert!(ptree.search(root, "crate::mod::c::BB").is_none());
        }

        #[test]
        fn test_import_multiple_namespaces() {
            let entry = "/mod.rs";
            let code = r#"
            mod a {
                pub mod x {}  // Type namespace
                pub fn x() {} // Value namespace
            }

            mod b {
                pub mod x {}  // Type namespace
                pub fn x() {} // Value namespace
            }

            use a::x;     // brings 'mod' and 'fn' inside 'a'
            pub use a::*; // same as above
            use b::*;     // brings 'mod' and 'fn' inside 'b'
            "#;

            let gcx = GlobalCx::default();
            gcx.configure(Config {
                load: ConfigLoad::empty(),
            });
            let sem = prepare(&gcx, [(entry, code)]).analyze(entry).unwrap();
            let ptree = &sem.ptree;
            let crate_ = ptree.crate_name();

            let vis_node = pnode!(ptree, "{crate_}::mod");
            let node_a_x = pnode!(ptree, "{crate_}::mod::a::x");
            let node_b_x = pnode!(ptree, "{crate_}::mod::b::x");

            let mut expected = vec![
                (NodeIndex(0), node_a_x.to_path_id(0)),
                (NodeIndex(0), node_a_x.to_path_id(1)),
                (vis_node, node_b_x.to_path_id(0)),
                (vis_node, node_b_x.to_path_id(1)),
            ];
            expected.sort_unstable();

            let node_x = pnode!(ptree, "{crate_}::mod::x");
            let mut use_values = ptree
                .node(node_x)
                .iter()
                .filter_map(|(_, item)| match item {
                    PubItem::Use(v) => Some((v.vis_node, v.dst)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            use_values.sort_unstable();

            assert_eq!(use_values, expected);
        }

        #[cfg(not(miri))] // Loading "core" takes too much time
        #[test]
        fn test_import_well_known_library() {
            let entry = "test";
            let code = r#"
            use ops::Add;
            use std::ops;
            "#;

            let gcx = GlobalCx::default();
            let sem = prepare(&gcx, [(entry, code)]).analyze(entry).unwrap();
            let ptree = &sem.ptree;
            let crate_ = ptree.crate_name();

            // use ops::Add;
            let add = pitem!(ptree, "{crate_}::{entry}::Add");
            assert_eq!(add.as_use().unwrap().dst, pid!(ptree, "core::ops::Add"));

            // use std::ops;
            let ops = pitem!(ptree, "{crate_}::{entry}::ops");
            assert_eq!(ops.as_use().unwrap().dst, pid!(ptree, "core::ops"));
        }

        #[test]
        fn test_import_custom_known_library() {
            let known = "known";
            let known_code = r#"
            pub struct T;
            impl T {
                pub const U: u32 = 0;
                pub const fn new() -> Self { Self }
            }
            "#;

            let entry = "test";
            let entry_code = r#"
            fn f() {
                let a = known::T::U;
                let b = known::T::new();
            }
            "#;

            let gcx = GlobalCx::default();
            gcx.configure(Config {
                load: ConfigLoad::empty(),
            });
            let mut analyzer = prepare(&gcx, [(known, known_code), (entry, entry_code)]);
            analyzer.set_known_library(known, known).unwrap();
            let sem = analyzer.analyze(entry).unwrap();
            let ptree = &sem.ptree;
            let crate_ = ptree.crate_name();

            // let a = known::T::U;
            let ia = pitem!(ptree, "{crate_}::{entry}::f::{{0}}::a");
            let ta = ptree.get_owned(ia.as_local().unwrap().tid);
            assert_eq!(
                ta,
                OwnedType::Path {
                    name: "u32".into(),
                    params: [].into()
                }
            );

            // let b = known::T::new();
            let ib = pitem!(ptree, "{crate_}::{entry}::f::{{0}}::b");
            let tb = ptree.get_owned(ib.as_local().unwrap().tid);
            assert!(matches!(tb, OwnedType::Path { name, .. } if name == "known::T"));
        }
    }

    mod visibility {
        use super::*;

        #[test]
        fn test_various_pub_visibility() {
            let entry = "test";
            let code = r#"
            mod a {
                mod b {
                    pub mod c {
                        pub struct T0; // Visible at root
                        pub(crate) struct T1; // Visible at crate
                        pub(in super::super) struct T2; // Visible at 'a'
                        pub(super) struct T3; // Visible at 'b'
                        struct T4; // Visible at 'c'
                    }
                }
            }
            "#;

            let gcx = GlobalCx::default();
            gcx.configure(Config {
                load: ConfigLoad::empty(),
            });
            let sem = prepare(&gcx, [(entry, code)]).analyze(entry).unwrap();
            let ptree = &sem.ptree;
            let crate_ = ptree.crate_name();

            // pub struct T0; // Visible at root
            let i0 = pitem!(ptree, "{crate_}::{entry}::a::b::c::T0");
            assert_eq!(i0.as_struct().unwrap().vis_node, pnode!(ptree, ""));

            // pub(crate) struct T1; // Visible at crate
            let i1 = pitem!(ptree, "{crate_}::{entry}::a::b::c::T1");
            assert_eq!(i1.as_struct().unwrap().vis_node, ptree.crate_node());

            // pub(in super::super) struct T2; // Visible at 'a'
            let i2 = pitem!(ptree, "{crate_}::{entry}::a::b::c::T2");
            assert_eq!(
                i2.as_struct().unwrap().vis_node,
                pnode!(ptree, "{crate_}::{entry}::a")
            );

            // pub(super) struct T3; // Visible at 'b'
            let i3 = pitem!(ptree, "{crate_}::{entry}::a::b::c::T3");
            assert_eq!(
                i3.as_struct().unwrap().vis_node,
                pnode!(ptree, "{crate_}::{entry}::a::b")
            );

            // struct T4; // Visible at 'c'
            let i4 = pitem!(ptree, "{crate_}::{entry}::a::b::c::T4");
            assert_eq!(
                i4.as_struct().unwrap().vis_node,
                pnode!(ptree, "{crate_}::{entry}::a::b::c")
            );
        }

        #[test]
        fn test_choosing_visible_item() {
            let entry = "test";
            let code = r#"
            mod a { struct A; }
            mod b { pub struct A; }
            mod c { struct A; }
            mod d {
                use super::{a::*, b::*, c::*};

                struct T {
                    a: A, // b::A
                }
            }
            "#;

            let gcx = GlobalCx::default();
            gcx.configure(Config {
                load: ConfigLoad::empty(),
            });
            let sem = prepare(&gcx, [(entry, code)]).analyze(entry).unwrap();
            let ptree = &sem.ptree;
            let crate_ = ptree.crate_name();

            let field = pitem!(ptree, "{crate_}::{entry}::d::T::a");
            let ty = ptree.get_owned(field.as_field().unwrap().tid);
            assert_eq!(
                ty,
                OwnedType::Path {
                    name: format!("{crate_}::{entry}::b::A"),
                    params: [OwnedParam::Self_].into(),
                }
            );
        }
    }

    mod chain {
        use super::*;

        #[test]
        fn test_shorten_type_alias_chain() {
            let entry = "test";
            let code = r#"
            mod a { pub struct A; }
            mod b { pub type B = super::a::A; }
            mod c { pub type C = (super::a::A, super::b::B); }
            mod d { pub type D = [super::c::C; 1]; }
            "#;

            let gcx = GlobalCx::default();
            gcx.configure(Config {
                load: ConfigLoad::empty(),
            });
            let sem = prepare(&gcx, [(entry, code)]).analyze(entry).unwrap();
            let ptree = &sem.ptree;
            let crate_ = ptree.crate_name();

            // mod a { pub struct A; }
            let ia = pitem!(ptree, "{crate_}::{entry}::a::A");
            let ta = ptree.get_owned(ia.as_struct().unwrap().tid);
            assert_eq!(
                ta,
                OwnedType::Path {
                    name: format!("{crate_}::{entry}::a::A"),
                    params: [OwnedParam::Self_].into(),
                }
            );

            // mod b { pub type B = super::a::A; }
            let ib = pitem!(ptree, "{crate_}::{entry}::b::B");
            let tb = ptree.get_owned(ib.as_type_alias().unwrap().tid);
            assert_eq!(tb, ta);

            // mod c { pub type C = (super::a::A, super::b::B); }
            let ic = pitem!(ptree, "{crate_}::{entry}::c::C");
            let tc = ptree.get_owned(ic.as_type_alias().unwrap().tid);
            assert_eq!(tc, OwnedType::Tuple([ta.clone(), ta.clone()].into()));

            // mod d { pub type D = [super::c::C; 1]; }
            let id = pitem!(ptree, "{crate_}::{entry}::d::D");
            let td = ptree.get_owned(id.as_type_alias().unwrap().tid);
            assert_eq!(
                td,
                OwnedType::Array {
                    elem: Box::new(tc.clone()),
                    len: ArrayLen::Fixed(1)
                }
            );
        }

        #[test]
        fn test_shorten_mixed_use_type_alias_chain() {
            let entry = "test";
            let code = r#"
            mod a { pub struct A; }
            mod b { pub type B = super::a::A; }
            mod c { pub use super::b::B as C; }
            mod d { pub type D = super::c::C; }
            mod e { pub use super::d::D; }
            "#;

            let gcx = GlobalCx::default();
            gcx.configure(Config {
                load: ConfigLoad::empty(),
            });
            let sem = prepare(&gcx, [(entry, code)]).analyze(entry).unwrap();
            let ptree = &sem.ptree;
            let crate_ = ptree.crate_name();

            // mod a { pub struct A; }
            let ia = pitem!(ptree, "{crate_}::{entry}::a::A");
            let ta = ptree.get_owned(ia.as_struct().unwrap().tid);
            assert_eq!(
                ta,
                OwnedType::Path {
                    name: format!("{crate_}::{entry}::a::A"),
                    params: [OwnedParam::Self_].into(),
                }
            );

            // mod b { pub type B = super::a::A; }
            let ib = pitem!(ptree, "{crate_}::{entry}::b::B");
            let tb = ptree.get_owned(ib.as_type_alias().unwrap().tid);
            assert_eq!(tb, ta);

            // mod c { pub use super::b::B as C; }
            let ic = pitem!(ptree, "{crate_}::{entry}::c::C");
            assert_eq!(
                ic.as_use().unwrap().dst,
                pid!(ptree, "{crate_}::{entry}::a::A")
            );

            // mod d { pub type D = super::c::C; }
            let id = pitem!(ptree, "{crate_}::{entry}::d::D");
            let td = ptree.get_owned(id.as_type_alias().unwrap().tid);
            assert_eq!(td, ta);

            // mod e { pub use super::d::D; }
            let ie = pitem!(ptree, "{crate_}::{entry}::e::D");
            assert_eq!(
                ie.as_use().unwrap().dst,
                pid!(ptree, "{crate_}::{entry}::a::A")
            );
        }
    }

    mod local {
        use super::*;

        #[test]
        fn test_local_fn_param() {
            let entry = "test";
            let code = r#"
            struct T { a: f32, b: f64 }

            fn f0(a: i32, b: i64) {}
            fn f1(a: [i32; 1]) {}
            fn f2(T { a, b }: T) {}
            fn f3(a: (i32, i64)) {}
            "#;

            let gcx = GlobalCx::default();
            gcx.configure(Config {
                load: ConfigLoad::empty(),
            });
            let sem = prepare(&gcx, [(entry, code)]).analyze(entry).unwrap();
            let ptree = &sem.ptree;
            let crate_ = ptree.crate_name();

            // fn f0(a: i32, b: i64) {}
            let ia = pitem!(ptree, "{crate_}::{entry}::f0::a");
            let ta = ptree.get_owned(ia.as_local().unwrap().tid);
            assert_eq!(
                ta,
                OwnedType::Path {
                    name: "i32".into(),
                    params: [].into()
                }
            );

            let ib = pitem!(ptree, "{crate_}::{entry}::f0::b");
            let tb = ptree.get_owned(ib.as_local().unwrap().tid);
            assert_eq!(
                tb,
                OwnedType::Path {
                    name: "i64".into(),
                    params: [].into()
                }
            );

            // fn f1(a: [i32; 1]) {}
            let ia = pitem!(ptree, "{crate_}::{entry}::f1::a");
            let ta = ptree.get_owned(ia.as_local().unwrap().tid);
            assert_eq!(
                ta,
                OwnedType::Array {
                    elem: Box::new(OwnedType::Path {
                        name: "i32".into(),
                        params: [].into()
                    }),
                    len: ArrayLen::Fixed(1),
                }
            );

            // struct T { a: f32, b: f64 }
            // fn f2(T { a, b }: T) {}
            let ia = pitem!(ptree, "{crate_}::{entry}::f2::a");
            let ta = ptree.get_owned(ia.as_local().unwrap().tid);
            assert_eq!(
                ta,
                OwnedType::Path {
                    name: "f32".into(),
                    params: [].into()
                }
            );

            let ib = pitem!(ptree, "{crate_}::{entry}::f2::b");
            let tb = ptree.get_owned(ib.as_local().unwrap().tid);
            assert_eq!(
                tb,
                OwnedType::Path {
                    name: "f64".into(),
                    params: [].into()
                }
            );

            // fn f3(a: (i32, i64)) {}
            let ia = pitem!(ptree, "{crate_}::{entry}::f3::a");
            let ta = ptree.get_owned(ia.as_local().unwrap().tid);
            assert_eq!(
                ta,
                OwnedType::Tuple(
                    [
                        OwnedType::Path {
                            name: "i32".into(),
                            params: [].into()
                        },
                        OwnedType::Path {
                            name: "i64".into(),
                            params: [].into()
                        }
                    ]
                    .into()
                )
            );
        }

        #[test]
        fn test_local_determined_by_fn_input_output() {
            let entry = "test";
            let code = r#"
            fn f() {
                fn f(a: u32) -> i32 { a as i32 }
                let a = 0;
                let b = f(a);
            }
            "#;

            let gcx = GlobalCx::default();
            gcx.configure(Config {
                load: ConfigLoad::empty(),
            });
            let sem = prepare(&gcx, [(entry, code)]).analyze(entry).unwrap();
            let ptree = &sem.ptree;
            let crate_ = ptree.crate_name();

            let ia = pitem!(ptree, "{crate_}::{entry}::f::{{0}}::a");
            let ta = ptree.get_owned(ia.as_local().unwrap().tid);
            assert_eq!(
                ta,
                OwnedType::Path {
                    name: "u32".into(),
                    params: [].into()
                }
            );

            let ib = pitem!(ptree, "{crate_}::{entry}::f::{{0}}::b");
            let tb = ptree.get_owned(ib.as_local().unwrap().tid);
            assert_eq!(
                tb,
                OwnedType::Path {
                    name: "i32".into(),
                    params: [].into()
                }
            );
        }

        #[test]
        fn test_local_determined_by_struct() {
            let entry = "test";
            let code = r#"
            fn f() {
                struct A(i32);
                let a = A(0);
            }
            "#;

            let gcx = GlobalCx::default();
            gcx.configure(Config {
                load: ConfigLoad::empty(),
            });
            let sem = prepare(&gcx, [(entry, code)]).analyze(entry).unwrap();
            let ptree = &sem.ptree;
            let crate_ = ptree.crate_name();

            let ia = pitem!(ptree, "{crate_}::{entry}::f::{{0}}::a");
            let ta = ptree.get_owned(ia.as_local().unwrap().tid);
            assert_eq!(
                ta,
                OwnedType::Path {
                    name: format!("{crate_}::{entry}::f::{{0}}::A"),
                    params: [
                        OwnedParam::Self_,
                        OwnedParam::Other {
                            name: "1".into(),
                            ty: OwnedType::Path {
                                name: "i32".into(),
                                params: [].into()
                            },
                        }
                    ]
                    .into(),
                }
            );
        }
    }

    mod etc {
        use super::*;

        #[cfg(not(miri))] // Loading "core" takes too much time
        #[test]
        fn test_array_length_in_type() {
            let entry = "test";
            let code = r#"
            // Complex array length
            const A: [i32; f0() + 2] = [0, 0, 0];
            const fn f0() -> usize { 1 }
            // Array length in a local type
            fn f1() {
                let a: [i32; 2] = [1, 2];
                let b = [0; 1];
            }
            "#;

            let gcx = GlobalCx::default();
            let sem = prepare(&gcx, [(entry, code)]).analyze(entry).unwrap();
            let ptree = &sem.ptree;
            let crate_ = ptree.crate_name();

            // const A: [i32; f0() + 2] = [0, 0, 0];
            let ia = pitem!(ptree, "{crate_}::{entry}::A");
            let ta = ptree.get_owned(ia.as_const().unwrap().type_id());
            let OwnedType::Array { elem, len } = ta else {
                panic!("{ta:?} is not an array");
            };
            assert_eq!(
                *elem,
                OwnedType::Path {
                    name: "i32".into(),
                    params: [].into()
                }
            );
            assert_eq!(len, ArrayLen::Fixed(3));

            // let a: [i32; 2] = [1, 2];
            let ia = pitem!(ptree, "{crate_}::{entry}::f1::{{0}}::a");
            let ta = ptree.get_owned(ia.as_local().unwrap().tid);
            let OwnedType::Array { elem, len } = ta else {
                panic!("{ta:?} is not an array");
            };
            assert_eq!(
                *elem,
                OwnedType::Path {
                    name: "i32".into(),
                    params: [].into()
                }
            );
            assert_eq!(len, ArrayLen::Fixed(2));

            // let b = [0; 1];
            let ib = pitem!(ptree, "{crate_}::{entry}::f1::{{0}}::b");
            let ta = ptree.get_owned(ib.as_local().unwrap().tid);
            let OwnedType::Array { elem, len } = ta else {
                panic!("{ta:?} is not an array");
            };
            assert_eq!(
                *elem,
                OwnedType::Path {
                    name: "int".into(),
                    params: [].into()
                }
            );
            assert_eq!(len, ArrayLen::Fixed(1));
        }

        #[cfg(not(miri))] // Loading "core" takes too much time
        #[test]
        fn test_constant() {
            use crate::{semantic::eval, Intern};

            let entry = "test";
            let code = r#"
            // A constant determined by function call.
            const A: i32 = double(2) + 1;
            const fn double(i: i32) -> i32 { i * 2 }

            // A constant determined by associated function call.
            const B: T = T::new(1);
            struct T(u32);
            impl T { const fn new(value: u32) -> Self { Self(value) } }

            // A zero sized constant determined by associated function call.
            const C: U = U::new();
            struct U;
            impl U { const fn new() -> Self { Self } }

            // A constant in a function.
            fn f() { const I: i32 = A + 3; }
            "#;

            let gcx = GlobalCx::default();
            let sem = prepare(&gcx, [(entry, code)]).analyze(entry).unwrap();
            let ptree = &sem.ptree;
            let evaluated = &sem.evaluated;
            let crate_ = ptree.crate_name();

            // const A: i32 = double(2) + 1;
            let pid = pid!(ptree, "{crate_}::{entry}::A");
            let v = evaluated.get_mapped_value_by_path_id(pid).unwrap();
            assert_eq!(v, &eval::Value::Scalar(eval::Scalar::I32(5)));

            // const B: T = T::new(1);
            let pid = pid!(ptree, "{crate_}::{entry}::B");
            let v = evaluated.get_mapped_value_by_path_id(pid).unwrap();
            assert_eq!(
                v,
                &eval::Value::Composed(vec![eval::Field {
                    name: gcx.intern_str("0"),
                    value: eval::Value::Scalar(eval::Scalar::U32(1))
                }])
            );

            // const C: U = U::new();
            let pid = pid!(ptree, "{crate_}::{entry}::C");
            let v = evaluated.get_mapped_value_by_path_id(pid).unwrap();
            assert_eq!(v, &eval::Value::Composed(vec![]));

            // const I: i32 = A + 3;
            let pid = pid!(ptree, "{crate_}::{entry}::f::{{0}}::I");
            let v = evaluated.get_mapped_value_by_path_id(pid).unwrap();
            assert_eq!(v, &eval::Value::Scalar(eval::Scalar::I32(8)));
        }

        #[test]
        fn test_impl_block() {
            let entry = "test";
            let code = r#"
            struct T;

            impl T {
                fn f0(self) {}
                fn f1(&self) {}
                fn f2(&mut self) {}
                fn f3() -> Self { Self }
            }

            mod a {
                mod b {
                    use super::super::T;
                    impl T {
                        pub(super) fn g0(self) {}
                    }
                }
            }
            "#;

            let gcx = GlobalCx::default();
            gcx.configure(Config {
                load: ConfigLoad::empty(),
            });
            let sem = prepare(&gcx, [(entry, code)]).analyze(entry).unwrap();
            let ptree = &sem.ptree;
            let crate_ = ptree.crate_name();

            // fn f0(self) {}
            let i0 = pitem!(ptree, "{crate_}::{entry}::T::f0::self");
            let t0 = ptree.get_owned(i0.as_local().unwrap().tid);
            assert!(matches!(
                t0,
                OwnedType::Path { name, .. } if name == format!("{crate_}::{entry}::T")
            ));

            // fn f1(&self) {}
            let i1 = pitem!(ptree, "{crate_}::{entry}::T::f1::self");
            let t1 = ptree.get_owned(i1.as_local().unwrap().tid);
            let OwnedType::Ref { elem } = t1 else {
                panic!("{t1:?} is not a reference");
            };
            assert!(matches!(
                *elem,
                OwnedType::Path { name, .. } if name == format!("{crate_}::{entry}::T")
            ));

            // fn f2(&mut self) {}
            let i2 = pitem!(ptree, "{crate_}::{entry}::T::f2::self");
            let t2 = ptree.get_owned(i2.as_local().unwrap().tid);
            let OwnedType::Ref { elem } = t2 else {
                panic!("{t2:?} is not a reference");
            };
            let OwnedType::Mut { elem } = *elem else {
                panic!("{elem:?} is not a mutable");
            };
            assert!(matches!(
                *elem,
                OwnedType::Path { name, .. } if name == format!("{crate_}::{entry}::T")
            ));

            // fn f3() -> Self { Self }
            let i3 = pitem!(ptree, "{crate_}::{entry}::T::f3");
            let t3 = ptree.get_owned(i3.as_fn().unwrap().tid);
            let OwnedType::Path { params, .. } = t3 else {
                panic!("{t3:?} is not a path");
            };
            assert!(matches!(
                &params[0],
                OwnedParam::Other { ty: OwnedType::Path { name, .. }, .. }
                if name == &format!("{crate_}::{entry}::T")
            ));

            // pub(super) fn g0(self) {}
            let g0 = pitem!(ptree, "{crate_}::{entry}::T::g0");
            assert_eq!(
                g0.as_fn().unwrap().vis_node,
                pnode!(ptree, "{crate_}::{entry}::a")
            );

            let i0 = pitem!(ptree, "{crate_}::{entry}::T::g0::self");
            let t0 = ptree.get_owned(i0.as_local().unwrap().tid);
            assert!(matches!(
                t0,
                OwnedType::Path { name, .. } if name == format!("{crate_}::{entry}::T")
            ));
        }
    }
}
