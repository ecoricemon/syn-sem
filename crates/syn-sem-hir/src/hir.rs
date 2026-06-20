use std::ops::{Index, IndexMut};

use crate::{
    lower, AssocItemId, BlockId, ExprId, FieldId, FileId, ItemId, LocalId, PatId, SignatureId,
    StmtId, TypeId, VariantId,
};
use syn_sem_ast as ast;
use syn_sem_common::{FilePath, InternedStr};
use syn_sem_name::{DefId, ImportId, Name, ScopeId};

/// HIR container produced for upper semantic phases.
#[derive(Debug, Default)]
pub struct Hir<'cx> {
    files: Vec<File<'cx>>,
    items: Vec<Item<'cx>>,
    signatures: Vec<Signature>,
    fields: Vec<Field<'cx>>,
    variants: Vec<Variant<'cx>>,
    assoc_items: Vec<AssocItem<'cx>>,
    blocks: Vec<Block<'cx>>,
    stmts: Vec<Stmt<'cx>>,
    locals: Vec<Local<'cx>>,
    pats: Vec<Pat<'cx>>,
    exprs: Vec<Expr<'cx>>,
    types: Vec<Type<'cx>>,
    body: lower::Body,
}

impl<'cx> Hir<'cx> {
    pub(crate) fn from_arena(arena: HirArena<'cx>) -> Self {
        let mut hir = Self {
            files: arena.files,
            items: arena.items,
            signatures: arena.signatures,
            fields: arena.fields,
            variants: arena.variants,
            assoc_items: arena.assoc_items,
            blocks: arena.blocks,
            stmts: arena.stmts,
            locals: arena.locals,
            pats: arena.pats,
            exprs: arena.exprs,
            types: arena.types,
            body: lower::Body::default(),
        };
        hir.body = lower::Body::from_hir(&hir);
        hir
    }

    /// Returns all HIR files.
    pub fn files(&self) -> &[File<'cx>] {
        &self.files
    }

    /// Returns all HIR item declarations.
    pub fn items(&self) -> &[Item<'cx>] {
        &self.items
    }

    /// Returns all HIR function-like signatures.
    pub fn signatures(&self) -> &[Signature] {
        &self.signatures
    }

    /// Returns all HIR fields.
    pub fn fields(&self) -> &[Field<'cx>] {
        &self.fields
    }

    /// Returns all HIR enum variants.
    pub fn variants(&self) -> &[Variant<'cx>] {
        &self.variants
    }

    /// Returns all HIR associated items.
    pub fn assoc_items(&self) -> &[AssocItem<'cx>] {
        &self.assoc_items
    }

    /// Returns all HIR braced source blocks.
    pub fn blocks(&self) -> &[Block<'cx>] {
        &self.blocks
    }

    /// Returns all HIR source statements.
    pub fn stmts(&self) -> &[Stmt<'cx>] {
        &self.stmts
    }

    /// Returns all HIR local `let` bindings.
    pub fn locals(&self) -> &[Local<'cx>] {
        &self.locals
    }

    /// Returns all HIR source patterns.
    pub fn pats(&self) -> &[Pat<'cx>] {
        &self.pats
    }

    /// Returns all HIR source expressions.
    pub fn exprs(&self) -> &[Expr<'cx>] {
        &self.exprs
    }

    /// Returns all HIR source types.
    pub fn types(&self) -> &[Type<'cx>] {
        &self.types
    }

    /// Returns lowered body facts derived from this HIR's source spine.
    pub fn body(&self) -> &lower::Body {
        &self.body
    }
}

pub(crate) struct HirArena<'cx> {
    pub(crate) files: Vec<File<'cx>>,
    pub(crate) items: Vec<Item<'cx>>,
    pub(crate) signatures: Vec<Signature>,
    pub(crate) fields: Vec<Field<'cx>>,
    pub(crate) variants: Vec<Variant<'cx>>,
    pub(crate) assoc_items: Vec<AssocItem<'cx>>,
    pub(crate) blocks: Vec<Block<'cx>>,
    pub(crate) stmts: Vec<Stmt<'cx>>,
    pub(crate) locals: Vec<Local<'cx>>,
    pub(crate) pats: Vec<Pat<'cx>>,
    pub(crate) exprs: Vec<Expr<'cx>>,
    pub(crate) types: Vec<Type<'cx>>,
}

impl<'cx> Index<FileId> for Hir<'cx> {
    type Output = File<'cx>;

    fn index(&self, id: FileId) -> &Self::Output {
        &self.files[id.index()]
    }
}

impl<'cx> Index<ItemId> for Hir<'cx> {
    type Output = Item<'cx>;

    fn index(&self, id: ItemId) -> &Self::Output {
        &self.items[id.index()]
    }
}

impl IndexMut<ItemId> for Hir<'_> {
    fn index_mut(&mut self, id: ItemId) -> &mut Self::Output {
        &mut self.items[id.index()]
    }
}

impl<'cx> Index<SignatureId> for Hir<'cx> {
    type Output = Signature;

    fn index(&self, id: SignatureId) -> &Self::Output {
        &self.signatures[id.index()]
    }
}

impl<'cx> Index<FieldId> for Hir<'cx> {
    type Output = Field<'cx>;

    fn index(&self, id: FieldId) -> &Self::Output {
        &self.fields[id.index()]
    }
}

impl<'cx> Index<VariantId> for Hir<'cx> {
    type Output = Variant<'cx>;

    fn index(&self, id: VariantId) -> &Self::Output {
        &self.variants[id.index()]
    }
}

impl<'cx> Index<AssocItemId> for Hir<'cx> {
    type Output = AssocItem<'cx>;

    fn index(&self, id: AssocItemId) -> &Self::Output {
        &self.assoc_items[id.index()]
    }
}

impl<'cx> Index<BlockId> for Hir<'cx> {
    type Output = Block<'cx>;

    fn index(&self, id: BlockId) -> &Self::Output {
        &self.blocks[id.index()]
    }
}

impl IndexMut<BlockId> for Hir<'_> {
    fn index_mut(&mut self, id: BlockId) -> &mut Self::Output {
        &mut self.blocks[id.index()]
    }
}

impl<'cx> Index<StmtId> for Hir<'cx> {
    type Output = Stmt<'cx>;

    fn index(&self, id: StmtId) -> &Self::Output {
        &self.stmts[id.index()]
    }
}

impl<'cx> Index<LocalId> for Hir<'cx> {
    type Output = Local<'cx>;

    fn index(&self, id: LocalId) -> &Self::Output {
        &self.locals[id.index()]
    }
}

impl<'cx> Index<PatId> for Hir<'cx> {
    type Output = Pat<'cx>;

    fn index(&self, id: PatId) -> &Self::Output {
        &self.pats[id.index()]
    }
}

impl<'cx> Index<ExprId> for Hir<'cx> {
    type Output = Expr<'cx>;

    fn index(&self, id: ExprId) -> &Self::Output {
        &self.exprs[id.index()]
    }
}

impl<'cx> Index<TypeId> for Hir<'cx> {
    type Output = Type<'cx>;

    fn index(&self, id: TypeId) -> &Self::Output {
        &self.types[id.index()]
    }
}

/// One represented source file.
#[derive(Debug)]
pub struct File<'cx> {
    /// File id in HIR.
    pub id: FileId,
    /// Interned file path.
    pub file_path: FilePath<'cx>,
    /// Top-level represented items in source order.
    pub items: Vec<ItemId>,
}

/// One represented item declaration.
#[derive(Debug)]
pub struct Item<'cx> {
    /// Item id in HIR.
    pub id: ItemId,
    /// Item name, when the item has one source-level name.
    pub name: Option<Name<'cx>>,
    /// Item visibility.
    pub visibility: Visibility<'cx>,
    /// Definition linked from the current name-resolution data, if available.
    pub def: Option<DefId>,
    /// Scope containing this item.
    pub parent_scope: Option<ScopeId>,
    /// Source-shaped item payload.
    pub kind: ItemKind<'cx>,
}

/// Source-shaped payload for an item declaration.
#[derive(Debug)]
pub enum ItemKind<'cx> {
    /// Constant item.
    Const {
        /// Constant type.
        ty_id: TypeId,
        /// Initializer expression.
        init: ExprId,
    },
    /// Enum item.
    Enum {
        /// Source generics.
        generics: Generics<'cx>,
        /// Represented variants.
        variants: Vec<VariantId>,
    },
    /// Function item.
    Fn {
        /// Source generics.
        generics: Generics<'cx>,
        /// Represented signature.
        signature: SignatureId,
        /// Function body block.
        block: BlockId,
    },
    /// Implementation block.
    Impl {
        /// Source generics.
        generics: Generics<'cx>,
        /// Implemented trait path, if this is a trait impl.
        trait_: Option<Vec<PathSegment<'cx>>>,
        /// Implementing self type.
        self_ty_id: TypeId,
        /// Represented associated items.
        items: Vec<AssocItemId>,
    },
    /// Module item.
    Mod {
        /// Whether this module contains its items inline.
        is_inline: bool,
        /// Scope used for module members, if linked from the current name-resolution data.
        scope: Option<ScopeId>,
        /// Inline child items represented under this module.
        items: Vec<ItemId>,
    },
    /// Struct item.
    Struct {
        /// Source generics.
        generics: Generics<'cx>,
        /// Represented fields.
        fields: Vec<FieldId>,
    },
    /// Trait item.
    Trait {
        /// Source generics.
        generics: Generics<'cx>,
        /// Represented associated items.
        items: Vec<AssocItemId>,
    },
    /// Type alias item.
    Type {
        /// Source generics.
        generics: Generics<'cx>,
        /// Aliased type.
        ty_id: TypeId,
    },
    /// Use item.
    Use {
        /// Imports created from this use item.
        imports: Vec<ImportId>,
    },
}

/// HIR-native item generics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generics<'cx> {
    /// Scope where generic parameters and bounds should be resolved.
    pub scope: Option<ScopeId>,
    /// Generic parameters in source order.
    pub params: Vec<GenericParam<'cx>>,
    /// Generic predicates from inline bounds and where-clauses.
    pub predicates: Vec<WherePredicate<'cx>>,
}

/// HIR-native generic parameter declared by an item.
///
/// A parameter introduces a generic slot on a declaration, such as `T` or `const N: usize` in
/// `struct Array<T, const N: usize>;`. Use-site values supplied to those slots are represented by
/// [`GenericArg`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericParam<'cx> {
    /// Type generic parameter.
    Type(TypeParam<'cx>),
    /// Const generic parameter.
    Const(ConstParam<'cx>),
    /// Unsupported generic parameter form.
    Unsupported,
}

/// HIR-native type generic parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParam<'cx> {
    /// Parameter name.
    pub name: Name<'cx>,
    /// Default type, when present.
    pub default_ty_id: Option<TypeId>,
}

/// HIR-native const generic parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstParam<'cx> {
    /// Parameter name.
    pub name: Name<'cx>,
    /// Parameter type.
    pub ty_id: TypeId,
}

/// HIR-native type parameter bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeParamBound<'cx> {
    /// Trait bound.
    Trait(Vec<PathSegment<'cx>>),
    /// Unsupported bound form.
    Unsupported,
}

/// HIR-native generic predicate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WherePredicate<'cx> {
    /// Type bound predicate.
    TypeBound {
        /// Type being constrained.
        subject_ty_id: TypeId,
        /// Bounds applied to the type.
        bounds: Vec<TypeParamBound<'cx>>,
    },
    /// Unsupported predicate form.
    Unsupported,
}

/// One represented function-like signature.
#[derive(Debug)]
pub struct Signature {
    /// Signature id in HIR.
    pub id: SignatureId,
    /// Source signature.
    pub source: SignatureSource,
    /// Signature parameters.
    ///
    /// This is always non-empty. `params[0]` is the output type and has no source pattern.
    /// `params[1..]` are input parameters in source order and have source patterns. Omitted
    /// function returns are represented as unit `()`, which is a tuple type with no element types;
    /// explicitly inferred returns use [`TypeKind::Infer`].
    pub params: Vec<SignatureParam>,
}

/// One represented function signature parameter.
#[derive(Debug)]
pub struct SignatureParam {
    /// Parameter type.
    pub ty_id: TypeId,
    /// Source pattern for this parameter.
    ///
    /// This is `None` for the output parameter at `Signature::params[0]` and `Some` for input
    /// parameters at `Signature::params[1..]`.
    pub pat: Option<PatId>,
}

/// One represented pattern occurrence.
#[derive(Debug)]
pub struct Pat<'cx> {
    /// Pattern id in HIR.
    pub id: PatId,
    /// Original semantic AST pattern.
    pub pat: &'cx ast::Pat<'cx>,
    /// HIR-native source pattern shape.
    pub kind: PatKind<'cx>,
    /// Scope containing this pattern.
    pub scope: Option<ScopeId>,
}

/// HIR-native source pattern shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatKind<'cx> {
    /// Identifier binding pattern.
    Ident {
        /// Bound identifier.
        name: Name<'cx>,
        /// Local definition introduced by this binding pattern.
        def: Option<DefId>,
        /// Whether the binding uses `ref`.
        is_ref: bool,
        /// Whether the binding is mutable.
        is_mut: bool,
    },
    /// Reference pattern.
    Reference {
        /// Referenced pattern.
        pat: PatId,
        /// Whether the reference pattern is mutable.
        is_mut: bool,
    },
    /// Path pattern.
    Path(Path<'cx>),
    /// Struct pattern.
    Struct {
        /// Path naming the matched struct.
        path: Vec<PathSegment<'cx>>,
        /// Field patterns.
        fields: Vec<PatStructField<'cx>>,
        /// Whether the pattern uses a rest marker.
        has_rest: bool,
    },
    /// Tuple pattern.
    Tuple {
        /// Element patterns.
        elems: Vec<PatId>,
    },
    /// Type-annotated pattern.
    Type {
        /// Inner pattern.
        pat: PatId,
        /// Annotated type.
        ty_id: TypeId,
    },
    /// Pattern form not represented natively yet.
    ///
    /// TODO: Add literal, rest, slice, and other unsupported pattern variants when an upper-phase
    /// consumer needs their structure.
    Unsupported,
}

/// One field pattern inside a represented struct pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatStructField<'cx> {
    /// Field member being matched.
    pub member: Name<'cx>,
    /// Pattern for the field value.
    pub pat: PatId,
}

/// Source role for a represented signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureSource {
    /// Free function signature.
    ItemFn,
    /// Impl associated function signature.
    ImplFn,
    /// Trait associated function signature.
    TraitFn,
    /// Closure signature.
    Closure,
}

/// One represented field declaration.
#[derive(Debug)]
pub struct Field<'cx> {
    /// Field id in HIR.
    pub id: FieldId,
    /// Field name.
    pub name: Name<'cx>,
    /// Field visibility.
    pub visibility: Visibility<'cx>,
    /// Field type.
    pub ty_id: TypeId,
    /// Source field kind.
    pub source: FieldSource,
}

/// Source for a represented field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldSource {
    /// Struct field.
    Struct,
    /// Enum variant field.
    Variant,
}

/// One represented enum variant declaration.
#[derive(Debug)]
pub struct Variant<'cx> {
    /// Variant id in HIR.
    pub id: VariantId,
    /// Definition linked from the current name-resolution data, if available.
    pub def: Option<DefId>,
    /// Variant name.
    pub name: Name<'cx>,
    /// Represented payload fields.
    pub fields: Vec<FieldId>,
    /// Discriminant expression, if present.
    pub discriminant: Option<ExprId>,
}

/// One represented associated item declaration.
#[derive(Debug)]
pub struct AssocItem<'cx> {
    /// Associated item id in HIR.
    pub id: AssocItemId,
    /// Associated item name.
    pub name: Name<'cx>,
    /// Definition linked from the current name-resolution data, if available.
    pub def: Option<DefId>,
    /// Source-shaped associated item payload.
    pub kind: AssocItemKind,
}

/// Source-shaped payload for an associated item declaration.
#[derive(Debug)]
pub enum AssocItemKind {
    /// Impl associated const.
    ImplConst {
        /// Associated const type.
        ty_id: TypeId,
        /// Initializer expression.
        init: ExprId,
    },
    /// Impl associated function.
    ImplFn {
        /// Represented signature.
        signature: SignatureId,
        /// Function body block.
        block: BlockId,
    },
    /// Impl associated type.
    ImplType {
        /// Assigned type.
        ty_id: TypeId,
    },
    /// Trait associated const.
    TraitConst {
        /// Associated const type.
        ty_id: TypeId,
        /// Optional default expression.
        default: Option<ExprId>,
    },
    /// Trait associated function.
    TraitFn {
        /// Represented signature.
        signature: SignatureId,
        /// Optional default body block.
        default: Option<BlockId>,
    },
    /// Trait associated type.
    TraitType {
        /// Optional default type.
        default_ty_id: Option<TypeId>,
    },
}

/// Visibility for represented declarations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility<'cx> {
    /// Public visibility.
    Public,
    /// Restricted visibility path, such as `crate` or `foo::bar`.
    Restricted(VisibilityPath<'cx>),
    /// Inherited private visibility.
    Private,
}

/// Generic-argument-free source path used by restricted visibility.
///
/// Rust visibility restrictions such as `pub(crate)` and `pub(in a::b)` accept plain path
/// segments, not generic arguments, so this intentionally stores names only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibilityPath<'cx> {
    /// Path segment names in source order.
    pub segments: Vec<Name<'cx>>,
}

impl<'cx> VisibilityPath<'cx> {
    fn from_ast(path: &ast::Path<'cx>) -> Self {
        Self {
            segments: path
                .segments
                .iter()
                .map(|segment| segment.ident.inner)
                .collect(),
        }
    }
}

impl<'cx> Visibility<'cx> {
    pub(crate) fn from_ast(visibility: &ast::Visibility<'cx>) -> Self {
        match visibility {
            ast::Visibility::Public(_) => Self::Public,
            ast::Visibility::Restricted(path) => Self::Restricted(VisibilityPath::from_ast(path)),
            ast::Visibility::Private => Self::Private,
        }
    }
}

pub(crate) fn item_visibility<'cx>(item: &'cx ast::Item<'cx>) -> Visibility<'cx> {
    match item {
        ast::Item::Const(item) => Visibility::from_ast(&item.vis),
        ast::Item::Enum(item) => Visibility::from_ast(&item.vis),
        ast::Item::Fn(item) => Visibility::from_ast(&item.vis),
        ast::Item::Impl(_) => Visibility::Private,
        ast::Item::Mod(item) => Visibility::from_ast(&item.vis),
        ast::Item::Struct(item) => Visibility::from_ast(&item.vis),
        ast::Item::Trait(item) => Visibility::from_ast(&item.vis),
        ast::Item::Type(item) => Visibility::from_ast(&item.vis),
        ast::Item::Use(item) => Visibility::from_ast(&item.vis),
    }
}

/// One braced source block.
///
/// This is currently a source-spine anchor. As body lowering grows enough to own block statement
/// facts directly, this type may shrink to source/scope linkage or be replaced by lowered body
/// queries for non-diagnostic consumers.
#[derive(Debug)]
pub struct Block<'cx> {
    /// Block id in HIR.
    pub id: BlockId,
    /// Original semantic AST block.
    pub block: &'cx ast::Block<'cx>,
    /// Represented statements in source order.
    pub stmts: Vec<StmtId>,
    /// Scope containing block-local bindings, if linked from the current name-resolution data.
    pub scope: Option<ScopeId>,
}

/// One represented statement occurrence.
///
/// This is currently a source-spine anchor. Statement order and lowered local/item/expression
/// facts are expected to move behind [`lower::Body`] as that model matures.
#[derive(Debug)]
pub struct Stmt<'cx> {
    /// Statement id in HIR.
    pub id: StmtId,
    /// Original semantic AST statement.
    pub stmt: &'cx ast::Stmt<'cx>,
    /// HIR-native source statement shape.
    pub kind: StmtKind,
    /// Scope containing this statement.
    pub scope: Option<ScopeId>,
}

/// HIR-native source statement shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StmtKind {
    /// Local `let` binding statement.
    Local(LocalId),
    /// Block-local item statement.
    Item(ItemId),
    /// Expression statement.
    Expr {
        /// Expression being evaluated.
        expr: ExprId,
        /// Whether this expression statement has a trailing semicolon.
        has_semi: bool,
    },
}

/// One represented local `let` binding.
///
/// This is currently a source-spine anchor. Lowered local facts such as binding definitions and
/// initializer expressions are expected to live behind [`lower::Local`] for upper phases.
#[derive(Debug)]
pub struct Local<'cx> {
    /// Local id in HIR.
    pub id: LocalId,
    /// Original semantic AST local binding.
    pub local: &'cx ast::Local<'cx>,
    /// Source pattern for this local binding.
    pub pat: PatId,
    /// Optional initializer expression.
    pub init: Option<ExprId>,
    /// Scope containing this local binding.
    pub scope: Option<ScopeId>,
}

/// One represented expression occurrence.
#[derive(Debug)]
pub struct Expr<'cx> {
    /// Expression id in HIR.
    pub id: ExprId,
    /// Original semantic AST expression.
    pub expr: &'cx ast::Expr<'cx>,
    /// HIR-native source expression shape.
    pub kind: ExprKind<'cx>,
    /// Scope used to resolve paths inside this expression occurrence.
    pub scope: Option<ScopeId>,
}

/// HIR-native source expression shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind<'cx> {
    /// Array literal expression.
    Array {
        /// Element expressions.
        elems: Vec<ExprId>,
    },
    /// Assignment expression.
    Assign {
        /// Left-hand side expression.
        left: ExprId,
        /// Right-hand side expression.
        right: ExprId,
    },
    /// Binary operator expression.
    Binary {
        /// Left-hand side expression.
        left: ExprId,
        /// Right-hand side expression.
        right: ExprId,
    },
    /// Block expression.
    Block {
        /// Block body.
        block: BlockId,
    },
    /// Function call expression.
    Call {
        /// Callee expression.
        func: ExprId,
        /// Call arguments.
        args: Vec<ExprId>,
    },
    /// Cast expression.
    Cast {
        /// Expression being cast.
        expr: ExprId,
        /// Target type.
        ty_id: TypeId,
    },
    /// Closure expression.
    Closure {
        /// Closure signature.
        signature: SignatureId,
        /// Closure body expression.
        body: ExprId,
    },
    /// Const block expression.
    Const {
        /// Const block body.
        block: BlockId,
    },
    /// Field access expression.
    Field {
        /// Base expression.
        base: ExprId,
        /// Field member name or tuple index.
        member: Name<'cx>,
    },
    /// Indexing expression.
    Index {
        /// Indexed expression.
        expr: ExprId,
        /// Index expression.
        index: ExprId,
    },
    /// Literal expression.
    Lit(Lit<'cx>),
    /// Method call expression.
    MethodCall {
        /// Receiver expression.
        receiver: ExprId,
        /// Method name.
        method: Name<'cx>,
        /// Method arguments.
        args: Vec<ExprId>,
    },
    /// Parenthesized expression.
    Paren {
        /// Inner expression.
        expr: ExprId,
    },
    /// Path expression.
    Path(Path<'cx>),
    /// Reference expression.
    Reference {
        /// Referenced expression.
        expr: ExprId,
        /// Whether the reference is mutable.
        is_mut: bool,
    },
    /// Repeated array expression.
    Repeat {
        /// Repeated value expression.
        expr: ExprId,
        /// Length expression.
        len: ExprId,
    },
    /// Return expression.
    Return {
        /// Optional returned expression.
        expr: Option<ExprId>,
    },
    /// Struct literal expression.
    Struct {
        /// Path naming the constructed struct.
        path: Vec<PathSegment<'cx>>,
        /// Field initializers.
        fields: Vec<ExprStructField<'cx>>,
        /// Optional rest expression.
        rest: Option<ExprId>,
    },
    /// Tuple expression.
    Tuple {
        /// Element expressions.
        elems: Vec<ExprId>,
    },
    /// Unary operator expression.
    Unary {
        /// Operand expression.
        expr: ExprId,
    },
}

/// One field initializer inside a represented struct expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprStructField<'cx> {
    /// Field member being initialized.
    pub member: Name<'cx>,
    /// Initializer expression.
    pub expr: ExprId,
}

/// One HIR type occurrence.
#[derive(Debug)]
pub struct Type<'cx> {
    /// Type id in HIR.
    pub id: TypeId,
    /// Original semantic AST type, when this type came directly from source syntax.
    ///
    /// This is `None` for synthetic types introduced by HIR lowering.
    pub ty: Option<&'cx ast::Type<'cx>>,
    /// HIR-native source type shape.
    pub kind: TypeKind<'cx>,
    /// Scope used to resolve paths inside this type occurrence.
    pub scope: Option<ScopeId>,
    /// Source role for this type occurrence.
    pub source: TypeSource,
}

/// HIR-native source type shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeKind<'cx> {
    /// Fixed-length array type.
    Array {
        /// Element type.
        elem_id: TypeId,
        /// Array length expression shape.
        len: ArrayLen,
    },
    /// Inferred type placeholder.
    Infer,
    /// Path type.
    Path(Path<'cx>),
    /// Borrowed reference type.
    Reference {
        /// Referenced type.
        elem_id: TypeId,
        /// Whether the reference is mutable.
        is_mut: bool,
    },
    /// Dynamically sized slice type.
    Slice {
        /// Element type.
        elem_id: TypeId,
    },
    /// Tuple type.
    Tuple {
        /// Tuple element types.
        elem_ids: Vec<TypeId>,
    },
}

/// HIR-native source path in type position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path<'cx> {
    /// Qualified self type, when the source type used qualified path syntax.
    pub qself: Option<QSelf<'cx>>,
    /// Path segments naming the type.
    ///
    /// For qualified paths, this remains the full source path after `as`; for example,
    /// `<T as a::b::Trait>::Assoc` stores `a::b::Trait::Assoc` here, while `qself.trait_path`
    /// stores only `a::b::Trait`.
    pub segments: Vec<PathSegment<'cx>>,
}

/// HIR-native qualified self type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QSelf<'cx> {
    /// Self type: `T` in `<T as a::b::Trait>::Assoc`.
    pub self_ty_id: TypeId,
    /// Trait path segments: `a::b::Trait` in `<T as a::b::Trait>::Assoc`.
    ///
    /// This is empty when the source used `<T>::Assoc` without an explicit trait path.
    pub trait_path: Vec<PathSegment<'cx>>,
}

/// One HIR-native type path segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSegment<'cx> {
    /// Segment name.
    pub name: Name<'cx>,
    /// Generic arguments on this segment.
    pub args: Vec<GenericArg<'cx>>,
}

/// HIR-native generic argument supplied at a use site.
///
/// An argument fills a generic slot declared by [`GenericParam`], such as `u8` or `3` in `Array<u8,
/// 3>`. Associated type and const constraints inside path arguments, such as `Iterator<Item = T>`
/// or `Trait<PANIC = false>`, are represented here as well.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericArg<'cx> {
    /// Type argument.
    Type(TypeId),
    /// Const expression argument.
    Const(ConstArg<'cx>),
    /// Associated type equality.
    AssocType {
        /// Associated type name.
        name: Name<'cx>,
        /// Assigned type.
        ty_id: TypeId,
    },
    /// Associated const equality.
    AssocConst {
        /// Associated const name.
        name: Name<'cx>,
        /// Assigned const value.
        value: ConstArg<'cx>,
    },
    /// Associated type constraint.
    Constraint {
        /// Associated type name.
        name: Name<'cx>,
        /// Source bounds.
        bounds: Vec<TypeParamBound<'cx>>,
    },
    /// Unsupported argument form.
    Unsupported,
}

/// Array length represented without owning expression lowering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayLen {
    /// Length expression.
    Expr(ExprId),
}

/// HIR-native const argument shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstArg<'cx> {
    /// Literal const argument.
    Lit(Lit<'cx>),
    /// Path const argument.
    Path(Path<'cx>),
    /// Const expression.
    Expr(ExprId),
}

/// HIR-native literal shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lit<'cx> {
    /// Integer literal stored as normalized base-10 digits.
    Int(InternedStr<'cx>),
    /// Floating-point literal stored as normalized base-10 digits.
    Float(InternedStr<'cx>),
    /// Boolean literal.
    Bool(bool),
}

/// Source role for a HIR type occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeSource {
    /// Constant item type, for example `T` in `const C: T = value;`.
    ConstType,
    /// Function signature parameter type.
    ///
    /// `index == 0` is the output type, for example `R` in `fn f() -> R`. `index >= 1` is an
    /// input parameter type, for example `T` in `fn f(x: T)`.
    SignatureParam {
        /// Parameter index in the represented signature.
        index: usize,
    },
    /// Impl self type, for example `T` in `impl T {}`.
    ImplSelf,
    /// Struct field type, for example `T` in `struct S { field: T }`.
    StructField,
    /// Enum variant field type, for example `T` in `enum E { V(T) }`.
    VariantField,
    /// Type alias target, for example `T` in `type Alias = T;`.
    TypeAlias,
    /// Associated const type, for example `T` in `const C: T = value;` inside a trait or impl.
    AssocConstType,
    /// Associated type value, for example `T` in `type Item = T;` inside a trait or impl.
    AssocTypeValue,
    /// Type generic parameter default, for example `T` in `struct S<U = T>;`.
    GenericParamDefault,
    /// Const generic parameter type, for example `usize` in `struct S<const N: usize>;`.
    ConstGenericParam,
    /// Where-predicate subject, for example `T` in `where T: Trait`.
    WherePredicateSubject,
    /// Nested type inside another HIR type occurrence, for example `T` in `Vec<T>`.
    Nested,
}
