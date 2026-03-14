//! # Taks id
//!
//! Each task contains its unique identification. Tasks could be rescheduled, so we could count up
//! the rescheduling for the same task then make panics when the rescheduling has happened too
//! many.

use crate::{
    ds::{self, queue::OnceQueue, tree::Tree},
    semantic::{
        entry::GlobalCx,
        tree::{
            NodeIndex, PathId, RawConst, RawEnum, RawField, RawFn, RawLocal, RawMod, RawStruct,
            RawTrait, RawTypeAlias, RawUse, RawVariant, TypeId,
        },
    },
    syntax::common::{IdentifySyn, SynId},
    TriResult,
};
use any_intern::Interned;
use std::{array, collections::vec_deque, fmt, iter, path::PathBuf};

pub(super) trait GenerateTask<'gcx> {
    type Output: Iterator<Item = Task<'gcx>>;

    fn generate_task(&self, pid: PathId) -> Self::Output;
}

impl<'gcx> GenerateTask<'gcx> for RawConst {
    type Output = array::IntoIter<Task<'gcx>, 4>;

    #[rustfmt::skip]
    fn generate_task(&self, pid: PathId) -> Self::Output {
        [
            Task::Resolve(TaskResolve::Const(TaskResolveConst::ResolveVis(pid))),
            Task::Resolve(TaskResolve::Const(TaskResolveConst::ResolveType(pid))),
            Task::FixType(TaskFixType::Const(pid)),
            Task::FindKnownLib(TaskFindKnownLibFrom::Type(self.syn_type().syn_id())),
        ]
        .into_iter()
    }
}

impl<'gcx> GenerateTask<'gcx> for RawEnum {
    type Output = array::IntoIter<Task<'gcx>, 2>;

    #[rustfmt::skip]
    fn generate_task(&self, pid: PathId) -> Self::Output {
        [
            Task::Resolve(TaskResolve::Enum(TaskResolveEnum::ResolveVis(pid))),
            Task::Resolve(TaskResolve::Enum(TaskResolveEnum::ResolveType(pid))),
        ]
        .into_iter()
    }
}

impl<'gcx> GenerateTask<'gcx> for RawField {
    type Output = array::IntoIter<Task<'gcx>, 4>;

    #[rustfmt::skip]
    fn generate_task(&self, pid: PathId) -> Self::Output {
        [
            Task::Resolve(TaskResolve::Field(TaskResolveField::ResolveVis(pid))),
            Task::Resolve(TaskResolve::Field(TaskResolveField::ResolveType(pid))),
            Task::FixType(TaskFixType::Field(pid)),
            Task::FindKnownLib(TaskFindKnownLibFrom::Type(self.as_syn().ty.syn_id())),
        ]
        .into_iter()
    }
}

impl<'gcx> GenerateTask<'gcx> for RawFn {
    type Output = array::IntoIter<Task<'gcx>, 5>;

    #[rustfmt::skip]
    fn generate_task(&self, pid: PathId) -> Self::Output {
        [
            Task::Resolve(TaskResolve::Fn(TaskResolveFn::ResolveVis(pid))),
            Task::Resolve(TaskResolve::Fn(TaskResolveFn::ResolveType(pid))),
            Task::FixType(TaskFixType::Fn(pid)),
            Task::FindKnownLib(TaskFindKnownLibFrom::Sig(self.as_syn_sig().syn_id())),
            Task::FindKnownLib(TaskFindKnownLibFrom::Block(self.as_syn_block().syn_id())),
        ]
        .into_iter()
    }
}

impl<'gcx> GenerateTask<'gcx> for RawLocal {
    type Output = array::IntoIter<Task<'gcx>, 2>;

    #[rustfmt::skip]
    fn generate_task(&self, pid: PathId) -> Self::Output {
        [
            Task::Resolve(TaskResolve::Local(TaskResolveLocal::ResolveType(pid))),
            Task::FixType(TaskFixType::Local(pid)),
        ]
        .into_iter()
    }
}

impl<'gcx> GenerateTask<'gcx> for RawMod {
    type Output = array::IntoIter<Task<'gcx>, 1>;

    #[rustfmt::skip]
    fn generate_task(&self, pid: PathId) -> Self::Output {
        [
            Task::Resolve(TaskResolve::Mod(TaskResolveMod::ResolveVis(pid))),
        ]
        .into_iter()
    }
}

impl<'gcx> GenerateTask<'gcx> for RawStruct {
    type Output = array::IntoIter<Task<'gcx>, 3>;

    #[rustfmt::skip]
    fn generate_task(&self, pid: PathId) -> Self::Output {
        [
            Task::Resolve(TaskResolve::Struct(TaskResolveStruct::ResolveVis(pid))),
            Task::Resolve(TaskResolve::Struct(TaskResolveStruct::ResolveType(pid))),
            Task::FixType(TaskFixType::Struct(pid)),
        ]
        .into_iter()
    }
}

impl<'gcx> GenerateTask<'gcx> for RawTrait {
    type Output = array::IntoIter<Task<'gcx>, 1>;

    #[rustfmt::skip]
    fn generate_task(&self, pid: PathId) -> Self::Output {
        [
            Task::Resolve(TaskResolve::Trait(TaskResolveTrait::ResolveVis(pid)))
        ]
        .into_iter()
    }
}

impl<'gcx> GenerateTask<'gcx> for RawTypeAlias {
    type Output = array::IntoIter<Task<'gcx>, 4>;

    #[rustfmt::skip]
    fn generate_task(&self, pid: PathId) -> Self::Output {
        [
            Task::Resolve(TaskResolve::TypeAlias(TaskResolveTypeAlias::ResolveVis(pid))),
            Task::Resolve(TaskResolve::TypeAlias(TaskResolveTypeAlias::ResolveType(pid))),
            Task::FixType(TaskFixType::TypeAlias(pid)),
            Task::FindKnownLib(TaskFindKnownLibFrom::Type(self.as_syn().ty.syn_id())),
        ]
        .into_iter()
    }
}

impl<'gcx> GenerateTask<'gcx> for RawUse {
    type Output = array::IntoIter<Task<'gcx>, 2>;

    #[rustfmt::skip]
    fn generate_task(&self, pid: PathId) -> Self::Output {
        [
            Task::Resolve(TaskResolve::Use(TaskResolveUse::ResolveVis(pid))),
            Task::FindKnownLib(TaskFindKnownLibFrom::Name(self.npath.clone())),
        ]
        .into_iter()
    }
}

impl<'gcx> GenerateTask<'gcx> for RawVariant {
    type Output = array::IntoIter<Task<'gcx>, 2>;

    #[rustfmt::skip]
    fn generate_task(&self, pid: PathId) -> Self::Output {
        [
            Task::Resolve(TaskResolve::Variant(TaskResolveVariant::ResolveVis(pid))),
            Task::Resolve(TaskResolve::Variant(TaskResolveVariant::ResolveDisc(pid))),
        ]
        .into_iter()
    }
}

#[derive(Debug, Clone)]
pub(super) enum Task<'gcx> {
    ConstructPathTree(TaskConstructPathTree),
    FindKnownLib(TaskFindKnownLibFrom),
    LoadLogic(TaskLoadLogic),
    Resolve(TaskResolve),
    FixType(TaskFixType),
    EvalConst(TaskEvalConst),
    EvalExpr(TaskEvalExpr),
    Monomorphize(TaskMonomorphize),
    Dyn(TaskDyn<'gcx>),
}

impl<'gcx> Task<'gcx> {
    pub(super) fn construct_path_tree_for_file(fpath: PathBuf, npath: String) -> Self {
        Self::ConstructPathTree(TaskConstructPathTree::File { fpath, npath })
    }

    pub(super) fn construct_path_tree_for_impl(item_impl: SynId, base: NodeIndex) -> Self {
        Self::ConstructPathTree(TaskConstructPathTree::Impl { item_impl, base })
    }

    pub(super) fn load_logic_for_file(file: SynId) -> Self {
        Self::LoadLogic(TaskLoadLogic::ImplsInFile { file })
    }

    pub(super) fn load_logic_for_impl(item_impl: SynId, base: NodeIndex) -> Self {
        Self::LoadLogic(TaskLoadLogic::Impl { item_impl, base })
    }

    pub(super) fn fix_impl_type(
        ty: SynId,
        generics: SynId,
        self_ty: TypeId,
        base: NodeIndex,
    ) -> Self {
        Self::FixType(TaskFixType::ImplType(TaskFixImplType {
            ty,
            generics,
            self_ty,
            base,
        }))
    }

    pub(super) fn eval_const_free(const_pid: PathId) -> Self {
        Self::EvalConst(TaskEvalConst::Free { const_pid })
    }

    pub(super) fn eval_const_inher(expr: SynId, ty: SynId, base: NodeIndex) -> Self {
        Self::EvalConst(TaskEvalConst::Inher { expr, ty, base })
    }

    pub(super) fn eval_const_trait_default(expr: SynId, ty: SynId, base: NodeIndex) -> Self {
        Self::EvalConst(TaskEvalConst::TraitDefault { expr, ty, base })
    }

    pub(super) fn eval_const_trait_impl(expr: SynId, ty: SynId, base: NodeIndex) -> Self {
        Self::EvalConst(TaskEvalConst::TraitImpl { expr, ty, base })
    }

    pub(super) fn eval_expr(expr: SynId, base: NodeIndex) -> Self {
        Self::EvalExpr(TaskEvalExpr { expr, base })
    }

    pub(super) fn monomorphize_impl(item_impl: SynId, self_ty: Option<TypeId>) -> Self {
        Self::Monomorphize(TaskMonomorphize::Impl { item_impl, self_ty })
    }

    pub(super) fn dyn_<F>(f: F, id: Interned<'gcx, str>) -> Self
    where
        F: FnMut(TaskDynInput<'gcx>) -> TriResult<(), ()> + Clone + 'gcx,
    {
        Self::Dyn(TaskDyn {
            custom: Box::new(f),
            id,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum TaskId<'gcx> {
    ConstructPathTree(TaskConstructPathTreeId),
    FindKnownLib(TaskFindKnownLibFromId),
    LoadLogic(TaskLoadLogicId),
    Resolve(TaskResolveId),
    FixType(TaskFixTypeId),
    EvalConst(TaskEvalConstId),
    EvalExpr(TaskEvalExprId),
    Monomorphize(TaskMonomorphizeId),
    Dyn(TaskDynId<'gcx>),
}

impl<'gcx> From<&Task<'gcx>> for TaskId<'gcx> {
    fn from(value: &Task<'gcx>) -> Self {
        match value {
            Task::ConstructPathTree(v) => Self::ConstructPathTree(v.into()),
            Task::FindKnownLib(v) => Self::FindKnownLib(v.into()),
            Task::LoadLogic(v) => Self::LoadLogic(v.into()),
            Task::Resolve(v) => Self::Resolve(v.into()),
            Task::FixType(v) => Self::FixType(v.into()),
            Task::EvalConst(v) => Self::EvalConst(v.into()),
            Task::EvalExpr(v) => Self::EvalExpr(v.into()),
            Task::Monomorphize(v) => Self::Monomorphize(v.into()),
            Task::Dyn(v) => Self::Dyn(v.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TaskConstructPathTree {
    /// A task to construct path tree for a file.
    File { fpath: PathBuf, npath: String },
    /// A task to construct path tree for an impl block.
    Impl { item_impl: SynId, base: NodeIndex },
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum TaskConstructPathTreeId {
    File { fpath: PathBuf },
    Impl { item_impl: SynId },
}

impl From<&TaskConstructPathTree> for TaskConstructPathTreeId {
    fn from(value: &TaskConstructPathTree) -> Self {
        match value {
            TaskConstructPathTree::File { fpath, .. } => Self::File {
                fpath: fpath.clone(),
            },
            TaskConstructPathTree::Impl { item_impl, .. } => Self::Impl {
                item_impl: *item_impl,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TaskFindKnownLibFrom {
    Name(String),
    /// Syn id to a [`syn::Type`].
    Type(SynId),
    /// Syn id to a [`syn::Signature`].
    Sig(SynId),
    /// Syn id to a [`syn::Block`].
    Block(SynId),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum TaskFindKnownLibFromId {
    Name(String),
    Type(SynId),
    Sig(SynId),
    Block(SynId),
}

impl From<&TaskFindKnownLibFrom> for TaskFindKnownLibFromId {
    fn from(value: &TaskFindKnownLibFrom) -> Self {
        match value {
            TaskFindKnownLibFrom::Name(s) => Self::Name(s.clone()),
            TaskFindKnownLibFrom::Type(sid) => Self::Type(*sid),
            TaskFindKnownLibFrom::Sig(sid) => Self::Sig(*sid),
            TaskFindKnownLibFrom::Block(sid) => Self::Block(*sid),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TaskLoadLogic {
    /// Load logic clauses about `impl` blocks in a file.
    ImplsInFile {
        /// Syn id to a [`syn::File`].
        file: SynId,
    },
    /// Load logic clauses about a single `impl` block
    Impl {
        /// Syn id to a [`syn::ItemImpl`].
        item_impl: SynId,
        base: NodeIndex,
    },
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum TaskLoadLogicId {
    ImplsInFile { file: SynId },
    Impl { item_impl: SynId },
}

impl From<&TaskLoadLogic> for TaskLoadLogicId {
    fn from(value: &TaskLoadLogic) -> Self {
        match value {
            TaskLoadLogic::ImplsInFile { file } => Self::ImplsInFile { file: *file },
            TaskLoadLogic::Impl { item_impl, .. } => Self::Impl {
                item_impl: *item_impl,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TaskResolve {
    Const(TaskResolveConst),
    Enum(TaskResolveEnum),
    Field(TaskResolveField),
    Fn(TaskResolveFn),
    Local(TaskResolveLocal),
    Mod(TaskResolveMod),
    Struct(TaskResolveStruct),
    Trait(TaskResolveTrait),
    TypeAlias(TaskResolveTypeAlias),
    Use(TaskResolveUse),
    Variant(TaskResolveVariant),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveId {
    Const(TaskResolveConstId),
    Enum(TaskResolveEnumId),
    Field(TaskResolveFieldId),
    Fn(TaskResolveFnId),
    Local(TaskResolveLocalId),
    Mod(TaskResolveModId),
    Struct(TaskResolveStructId),
    Trait(TaskResolveTraitId),
    TypeAlias(TaskResolveTypeAliasId),
    Use(TaskResolveUseId),
    Variant(TaskResolveVariantId),
}

impl From<&TaskResolve> for TaskResolveId {
    fn from(value: &TaskResolve) -> Self {
        match value {
            TaskResolve::Const(v) => Self::Const(v.into()),
            TaskResolve::Enum(v) => Self::Enum(v.into()),
            TaskResolve::Field(v) => Self::Field(v.into()),
            TaskResolve::Fn(v) => Self::Fn(v.into()),
            TaskResolve::Local(v) => Self::Local(v.into()),
            TaskResolve::Mod(v) => Self::Mod(v.into()),
            TaskResolve::Struct(v) => Self::Struct(v.into()),
            TaskResolve::Trait(v) => Self::Trait(v.into()),
            TaskResolve::TypeAlias(v) => Self::TypeAlias(v.into()),
            TaskResolve::Use(v) => Self::Use(v.into()),
            TaskResolve::Variant(v) => Self::Variant(v.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveConst {
    ResolveVis(PathId),
    ResolveType(PathId),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveConstId {
    ResolveVis(PathId),
    ResolveType(PathId),
}

impl From<&TaskResolveConst> for TaskResolveConstId {
    fn from(value: &TaskResolveConst) -> Self {
        match value {
            TaskResolveConst::ResolveVis(pid) => Self::ResolveVis(*pid),
            TaskResolveConst::ResolveType(pid) => Self::ResolveType(*pid),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveEnum {
    ResolveVis(PathId),
    ResolveType(PathId),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveEnumId {
    ResolveVis(PathId),
    ResolveType(PathId),
}

impl From<&TaskResolveEnum> for TaskResolveEnumId {
    fn from(value: &TaskResolveEnum) -> Self {
        match value {
            TaskResolveEnum::ResolveVis(pid) => Self::ResolveVis(*pid),
            TaskResolveEnum::ResolveType(pid) => Self::ResolveType(*pid),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveField {
    ResolveVis(PathId),
    ResolveType(PathId),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveFieldId {
    ResolveVis(PathId),
    ResolveType(PathId),
}

impl From<&TaskResolveField> for TaskResolveFieldId {
    fn from(value: &TaskResolveField) -> Self {
        match value {
            TaskResolveField::ResolveVis(pid) => Self::ResolveVis(*pid),
            TaskResolveField::ResolveType(pid) => Self::ResolveType(*pid),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveFn {
    ResolveVis(PathId),
    ResolveType(PathId),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveFnId {
    ResolveVis(PathId),
    ResolveType(PathId),
}

impl From<&TaskResolveFn> for TaskResolveFnId {
    fn from(value: &TaskResolveFn) -> Self {
        match value {
            TaskResolveFn::ResolveVis(pid) => Self::ResolveVis(*pid),
            TaskResolveFn::ResolveType(pid) => Self::ResolveType(*pid),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveLocal {
    ResolveType(PathId),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveLocalId {
    ResolveType(PathId),
}

impl From<&TaskResolveLocal> for TaskResolveLocalId {
    fn from(value: &TaskResolveLocal) -> Self {
        match value {
            TaskResolveLocal::ResolveType(pid) => Self::ResolveType(*pid),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveMod {
    ResolveVis(PathId),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveModId {
    ResolveVis(PathId),
}

impl From<&TaskResolveMod> for TaskResolveModId {
    fn from(value: &TaskResolveMod) -> Self {
        match value {
            TaskResolveMod::ResolveVis(pid) => Self::ResolveVis(*pid),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveStruct {
    ResolveVis(PathId),
    ResolveType(PathId),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveStructId {
    ResolveVis(PathId),
    ResolveType(PathId),
}

impl From<&TaskResolveStruct> for TaskResolveStructId {
    fn from(value: &TaskResolveStruct) -> Self {
        match value {
            TaskResolveStruct::ResolveVis(pid) => Self::ResolveVis(*pid),
            TaskResolveStruct::ResolveType(pid) => Self::ResolveType(*pid),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveTrait {
    ResolveVis(PathId),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveTraitId {
    ResolveVis(PathId),
}

impl From<&TaskResolveTrait> for TaskResolveTraitId {
    fn from(value: &TaskResolveTrait) -> Self {
        match value {
            TaskResolveTrait::ResolveVis(pid) => Self::ResolveVis(*pid),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveTypeAlias {
    ResolveVis(PathId),
    ResolveType(PathId),
}
#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveTypeAliasId {
    ResolveVis(PathId),
    ResolveType(PathId),
}

impl From<&TaskResolveTypeAlias> for TaskResolveTypeAliasId {
    fn from(value: &TaskResolveTypeAlias) -> Self {
        match value {
            TaskResolveTypeAlias::ResolveVis(pid) => Self::ResolveVis(*pid),
            TaskResolveTypeAlias::ResolveType(pid) => Self::ResolveType(*pid),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveUse {
    ResolveVis(PathId),
    ResolveDst,
}

impl TaskResolveUse {
    pub(super) fn tasks_for_all<'gcx>() -> impl Iterator<Item = Task<'gcx>> {
        iter::once(Task::Resolve(TaskResolve::Use(Self::ResolveDst)))
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveUseId {
    ResolveVis(PathId),
    ResolveDst,
}

impl From<&TaskResolveUse> for TaskResolveUseId {
    fn from(value: &TaskResolveUse) -> Self {
        match value {
            TaskResolveUse::ResolveVis(pid) => Self::ResolveVis(*pid),
            TaskResolveUse::ResolveDst => Self::ResolveDst,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveVariant {
    ResolveVis(PathId),
    ResolveDisc(PathId),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum TaskResolveVariantId {
    ResolveVis(PathId),
    ResolveDisc(PathId),
}

impl From<&TaskResolveVariant> for TaskResolveVariantId {
    fn from(value: &TaskResolveVariant) -> Self {
        match value {
            TaskResolveVariant::ResolveVis(pid) => Self::ResolveVis(*pid),
            TaskResolveVariant::ResolveDisc(pid) => Self::ResolveDisc(*pid),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TaskFixType {
    Const(PathId),
    Field(PathId),
    Fn(PathId),
    Local(PathId),
    Struct(PathId),
    TypeAlias(PathId),
    ImplType(TaskFixImplType),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum TaskFixTypeId {
    Const(PathId),
    Field(PathId),
    Fn(PathId),
    Local(PathId),
    Struct(PathId),
    TypeAlias(PathId),
    ImplType(TaskFixImplTypeId),
}

impl From<&TaskFixType> for TaskFixTypeId {
    fn from(value: &TaskFixType) -> Self {
        match value {
            TaskFixType::Const(pid) => Self::Const(*pid),
            TaskFixType::Field(pid) => Self::Field(*pid),
            TaskFixType::Fn(pid) => Self::Fn(*pid),
            TaskFixType::Local(pid) => Self::Local(*pid),
            TaskFixType::Struct(pid) => Self::Struct(*pid),
            TaskFixType::TypeAlias(pid) => Self::TypeAlias(*pid),
            TaskFixType::ImplType(v) => Self::ImplType(v.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct TaskFixImplType {
    /// Syn id to a [`syn::Type`].
    pub(super) ty: SynId,
    /// Syn id to a [`syn::Generics`].
    pub(super) generics: SynId,
    pub(super) self_ty: TypeId,
    pub(super) base: NodeIndex,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) struct TaskFixImplTypeId {
    pub(super) ty: SynId,
}

impl From<&TaskFixImplType> for TaskFixImplTypeId {
    fn from(value: &TaskFixImplType) -> Self {
        Self { ty: value.ty }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TaskEvalConst {
    Free {
        const_pid: PathId,
    },
    Inher {
        /// Syn id to a [`syn::Expr`].
        expr: SynId,
        /// Syn id to a [`syn::Type`].
        ty: SynId,
        base: NodeIndex,
    },
    TraitDefault {
        /// Syn id to a [`syn::Expr`].
        expr: SynId,
        /// Syn id to a [`syn::Type`].
        ty: SynId,
        base: NodeIndex,
    },
    TraitImpl {
        /// Syn id to a [`syn::Expr`].
        expr: SynId,
        /// Syn id to a [`syn::Type`].
        ty: SynId,
        base: NodeIndex,
    },
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum TaskEvalConstId {
    Free { const_pid: PathId },
    Inher { expr: SynId },
    TraitDefault { expr: SynId },
    TraitImpl { expr: SynId },
}

impl From<&TaskEvalConst> for TaskEvalConstId {
    fn from(value: &TaskEvalConst) -> Self {
        match value {
            TaskEvalConst::Free { const_pid } => Self::Free {
                const_pid: *const_pid,
            },
            TaskEvalConst::Inher { expr, .. } => Self::Inher { expr: *expr },
            TaskEvalConst::TraitDefault { expr, .. } => Self::TraitDefault { expr: *expr },
            TaskEvalConst::TraitImpl { expr, .. } => Self::TraitImpl { expr: *expr },
        }
    }
}

// TODO: Dost this need type info like TaskEvalConst?
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct TaskEvalExpr {
    /// Syn id to a [`syn::Expr`].
    pub(super) expr: SynId,
    pub(super) base: NodeIndex,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) struct TaskEvalExprId {
    pub(super) expr: SynId,
}

impl From<&TaskEvalExpr> for TaskEvalExprId {
    fn from(value: &TaskEvalExpr) -> Self {
        Self { expr: value.expr }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum TaskMonomorphize {
    Impl {
        item_impl: SynId,
        self_ty: Option<TypeId>,
    },
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum TaskMonomorphizeId {
    Impl {
        item_impl: SynId,
        self_ty: Option<TypeId>,
    },
}

impl From<&TaskMonomorphize> for TaskMonomorphizeId {
    fn from(value: &TaskMonomorphize) -> Self {
        match value {
            TaskMonomorphize::Impl { item_impl, self_ty } => Self::Impl {
                item_impl: *item_impl,
                self_ty: *self_ty,
            },
        }
    }
}

pub(super) struct TaskDyn<'gcx> {
    pub(super) custom: Box<dyn Custom<'gcx> + 'gcx>,
    pub(super) id: Interned<'gcx, str>,
}

impl fmt::Debug for TaskDyn<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return f
            .debug_struct("TaskDyn")
            .field("custom", &Helper(&*self.custom))
            .field("id", &self.id)
            .finish();

        // === Internal helpers ===

        struct Helper<'a, 'gcx>(&'a dyn Custom<'gcx>);

        impl fmt::Debug for Helper<'_, '_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.debug(f)
            }
        }
    }
}

impl Clone for TaskDyn<'_> {
    fn clone(&self) -> Self {
        Self {
            custom: self.custom.clone(),
            id: self.id,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) struct TaskDynId<'gcx>(Interned<'gcx, str>);

impl<'gcx> From<&TaskDyn<'gcx>> for TaskDynId<'gcx> {
    fn from(value: &TaskDyn<'gcx>) -> Self {
        Self(value.id)
    }
}

pub(super) trait Custom<'gcx> {
    fn run(&mut self, input: TaskDynInput<'gcx>) -> TriResult<(), ()>;

    fn clone(&self) -> Box<dyn Custom<'gcx> + 'gcx>;

    fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("dyn task")
    }
}

impl<'gcx, F> Custom<'gcx> for F
where
    F: FnMut(TaskDynInput<'gcx>) -> TriResult<(), ()> + Clone + 'gcx,
{
    fn run(&mut self, input: TaskDynInput<'gcx>) -> TriResult<(), ()> {
        self(input)
    }

    fn clone(&self) -> Box<dyn Custom<'gcx> + 'gcx> {
        Box::new(self.clone())
    }
}

// Maybe we need more input someday
pub(super) struct TaskDynInput<'gcx> {
    pub(super) gcx: &'gcx GlobalCx<'gcx>,
}

#[derive(Debug)]
pub(super) struct TaskQueue<'gcx> {
    queue: OnceQueue<TaskItem<'gcx>>,
    relation: Tree<TaskNode<'gcx>>,
    just_popped: TaskNodeId,
}

impl<'gcx> TaskQueue<'gcx> {
    const ROOT_NODE: TaskNodeId = TaskNodeId(Tree::ROOT);

    pub(super) fn new() -> Self {
        let root = TaskNode {
            state: TaskState::Alive,
            setup: None,
            cleanup: None,
        };

        Self {
            queue: OnceQueue::new(),
            relation: Tree::new(root),
            just_popped: Self::ROOT_NODE,
        }
    }

    pub(super) fn len(&self) -> usize {
        self.queue.len()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(super) fn reset(&mut self) {
        self.queue.reset();
        self.relation.clear();
        self.just_popped = Self::ROOT_NODE;
    }

    pub(super) fn iter(&self) -> TaskIter<'_, 'gcx> {
        TaskIter {
            inner: self.queue.iter(),
        }
    }

    /// Appends the task at the end of the queue if the queue has not seen the task before.
    ///
    /// If the queue has seen the task, then returns it within error.
    pub(super) fn push_back(&mut self, task: Task<'gcx>) -> Result<TaskNodeId, Task<'gcx>> {
        self.push(task, |queue, task_item| queue.push_back(task_item))
    }

    /// Appends the task at the beginning of the queue if the queue has not seen the task before.
    ///
    /// If the queue has seen the task, then returns it within error.
    pub(super) fn push_front(&mut self, task: Task<'gcx>) -> Result<TaskNodeId, Task<'gcx>> {
        self.push(task, |queue, task_item| queue.push_front(task_item))
    }

    pub(super) fn push_back_force(&mut self, task: TaskItem<'gcx>) -> TaskNodeId {
        // We do not allow duplicate tasks at a time.
        debug_assert_eq!(self.queue.count(&task), 0);

        let TaskItem {
            task,
            this_node,
            parent_node,
            ..
        } = task;

        let this_node = if let Some(node) = self.relation.get_mut(this_node.0) {
            node.state = TaskState::Alive;
            this_node
        } else {
            let index = self
                .relation
                .insert(
                    parent_node.0,
                    TaskNode {
                        state: TaskState::Alive,
                        setup: None,
                        cleanup: None,
                    },
                )
                .unwrap();
            TaskNodeId(index)
        };

        self.queue.push_back_force(TaskItem {
            task,
            this_node,
            parent_node,
        });

        this_node
    }

    pub(super) fn pop_front(&mut self) -> Option<TaskItem<'gcx>> {
        let task_item = self.queue.pop_front()?;
        self.just_popped = task_item.this_node;
        Some(task_item)
    }

    pub(super) fn just_popped_task_node(&self) -> TaskNodeId {
        self.just_popped
    }

    pub(super) fn parent_task_node(&self, child: TaskNodeId) -> TaskNodeId {
        let index = self.relation.parent(child.0).unwrap();
        TaskNodeId(index)
    }

    /// # Panics
    ///
    /// Panics if the given node is invalid.
    pub(super) fn set_setup_task(
        &mut self,
        node_id: TaskNodeId,
        setup: Task<'gcx>,
    ) -> Option<Task<'gcx>> {
        let node = self.relation.get_mut(node_id.0).unwrap();
        node.setup.replace(setup)
    }

    /// # Panics
    ///
    /// Panics if the given node is invalid.
    pub(super) fn set_cleanup_task(
        &mut self,
        node_id: TaskNodeId,
        cleanup: Task<'gcx>,
    ) -> Option<Task<'gcx>> {
        let node = self.relation.get_mut(node_id.0).unwrap();
        node.cleanup.replace(cleanup)
    }

    /// # Panics
    ///
    /// Panics if the given node is invalid.
    pub(super) fn get_cleanup_task(&self, node_id: TaskNodeId) -> Option<&Task<'gcx>> {
        let node = self.relation.get(node_id.0).unwrap();
        node.cleanup.as_ref()
    }

    /// Finds setup tasks for the given task then appends them to the end of the given buffer.
    ///
    /// Caller is supposed to process setup tasks from the end to start of the buffer.
    ///
    /// # Panics
    ///
    /// Panics if the given node is invalid.
    pub(super) fn append_setup_tasks(&self, node_id: TaskNodeId, buf: &mut Vec<Task<'gcx>>) {
        if node_id == Self::ROOT_NODE {
            return;
        }

        // Setup must be done from the root node.
        let node = self.relation.get(node_id.0).unwrap();
        if let Some(task) = &node.setup {
            buf.push(task.clone());
        }
        let parent_id = self.parent_task_node(node_id);
        self.append_setup_tasks(parent_id, buf);
    }

    /// Finds cleanup tasks for the given task then appends them to the end of the given buffer.
    ///
    /// Caller is supposed to process cleanup tasks from the end to start of the buffer.
    ///
    /// # Panics
    ///
    /// Panics if the given node is invalid.
    pub(super) fn append_cleanup_tasks(&self, node_id: TaskNodeId, buf: &mut Vec<Task<'gcx>>) {
        if node_id == Self::ROOT_NODE {
            return;
        }

        // Cleanup must be done from this node.
        let node = self.relation.get(node_id.0).unwrap();
        let parent_id = self.parent_task_node(node_id);
        self.append_cleanup_tasks(parent_id, buf);
        if let Some(task) = &node.cleanup {
            buf.push(task.clone());
        }
    }

    pub(super) fn mark_done(&mut self, node_id: TaskNodeId) {
        let node = self.relation.get_mut(node_id.0).unwrap();
        node.state = TaskState::Done;

        // If all tasks in this sub-tree have been done all, then we can remove the sub-tree.
        if self
            .relation
            .traverse_from(node_id.0, |node| {
                match node.state {
                    TaskState::Done => None,      // Continue traversing
                    TaskState::Alive => Some(()), // Found a yet done node, Stop traversing
                }
            })
            .is_none()
        {
            self.relation.take(node_id.0);
        }
    }

    fn push<F>(&mut self, task: Task<'gcx>, push: F) -> Result<TaskNodeId, Task<'gcx>>
    where
        F: FnOnce(&mut OnceQueue<TaskItem<'gcx>>, TaskItem<'gcx>) -> Result<(), TaskItem<'gcx>>,
    {
        let index = self.relation.next_index();
        let this_node = TaskNodeId(index);
        let task_item = TaskItem {
            task,
            this_node,
            parent_node: self.just_popped,
        };
        match push(&mut self.queue, task_item) {
            Ok(()) => {
                self.relation
                    .insert(
                        self.just_popped.0,
                        TaskNode {
                            state: TaskState::Alive,
                            setup: None,
                            cleanup: None,
                        },
                    )
                    .unwrap();
                Ok(this_node)
            }
            Err(task_item) => Err(task_item.task),
        }
    }
}

impl<'a, 'gcx> IntoIterator for &'a TaskQueue<'gcx> {
    type Item = &'a Task<'gcx>;
    type IntoIter = TaskIter<'a, 'gcx>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Debug)]
struct TaskNode<'gcx> {
    state: TaskState,
    setup: Option<Task<'gcx>>,
    cleanup: Option<Task<'gcx>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskState {
    Alive,
    Done,
}

#[derive(Debug)]
pub(super) struct TaskItem<'gcx> {
    pub(super) task: Task<'gcx>,
    pub(super) this_node: TaskNodeId,
    pub(super) parent_node: TaskNodeId,
}

impl<'gcx> ds::queue::Identify for TaskItem<'gcx> {
    type Id = TaskId<'gcx>;

    fn id(&self) -> Self::Id {
        TaskId::from(&self.task)
    }
}

impl<'gcx> ds::queue::Identify for Task<'gcx> {
    type Id = TaskId<'gcx>;

    fn id(&self) -> Self::Id {
        TaskId::from(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub(super) struct TaskNodeId(ds::tree::NodeIndex);

pub(super) struct TaskIter<'a, 'gcx> {
    inner: vec_deque::Iter<'a, TaskItem<'gcx>>,
}

impl<'a, 'gcx> Iterator for TaskIter<'a, 'gcx> {
    type Item = &'a Task<'gcx>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|task_item| &task_item.task)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for TaskIter<'_, '_> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl DoubleEndedIterator for TaskIter<'_, '_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|task_item| &task_item.task)
    }
}

impl iter::FusedIterator for TaskIter<'_, '_> {}
