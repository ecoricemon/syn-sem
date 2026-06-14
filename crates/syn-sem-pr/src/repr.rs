use std::ops::{Index, IndexMut};

use crate::{AssocItemId, BlockId, FieldId, FileId, ItemId, SignatureId, TypeId, VariantId};
use syn_sem_ast as ast;
use syn_sem_common::{FilePath, InternedStr};
use syn_sem_name::{DefId, Name, ScopeId};

/// Rust source program representation produced for semantic phases.
#[derive(Debug, Default)]
pub struct ProgramRepr<'cx> {
    files: Vec<File<'cx>>,
    items: Vec<Item<'cx>>,
    signatures: Vec<Signature<'cx>>,
    fields: Vec<Field<'cx>>,
    variants: Vec<Variant<'cx>>,
    assoc_items: Vec<AssocItem<'cx>>,
    blocks: Vec<Block<'cx>>,
    types: Vec<Type<'cx>>,
}

impl<'cx> ProgramRepr<'cx> {
    /// Returns all represented files.
    pub fn files(&self) -> &[File<'cx>] {
        &self.files
    }

    /// Returns all represented item declarations.
    pub fn items(&self) -> &[Item<'cx>] {
        &self.items
    }

    /// Returns all represented function-like signatures.
    pub fn signatures(&self) -> &[Signature<'cx>] {
        &self.signatures
    }

    /// Returns all represented fields.
    pub fn fields(&self) -> &[Field<'cx>] {
        &self.fields
    }

    /// Returns all represented enum variants.
    pub fn variants(&self) -> &[Variant<'cx>] {
        &self.variants
    }

    /// Returns all represented associated items.
    pub fn assoc_items(&self) -> &[AssocItem<'cx>] {
        &self.assoc_items
    }

    /// Returns all represented braced source blocks.
    pub fn blocks(&self) -> &[Block<'cx>] {
        &self.blocks
    }

    /// Returns all represented source types.
    pub fn types(&self) -> &[Type<'cx>] {
        &self.types
    }

    pub(crate) fn next_file_id(&self) -> FileId {
        FileId::new(self.files.len())
    }

    pub(crate) fn add_file(&mut self, file: File<'cx>) {
        let id = file.id;
        assert_eq!(id, self.next_file_id());
        self.files.push(file);
    }

    pub(crate) fn next_item_id(&self) -> ItemId {
        ItemId::new(self.items.len())
    }

    pub(crate) fn add_item(&mut self, item: Item<'cx>) {
        let id = item.id;
        assert_eq!(id, self.next_item_id());
        self.items.push(item);
    }

    pub(crate) fn next_signature_id(&self) -> SignatureId {
        SignatureId::new(self.signatures.len())
    }

    pub(crate) fn add_signature(&mut self, signature: Signature<'cx>) {
        let id = signature.id;
        assert_eq!(id, self.next_signature_id());
        self.signatures.push(signature);
    }

    pub(crate) fn next_field_id(&self) -> FieldId {
        FieldId::new(self.fields.len())
    }

    pub(crate) fn add_field(&mut self, field: Field<'cx>) {
        let id = field.id;
        assert_eq!(id, self.next_field_id());
        self.fields.push(field);
    }

    pub(crate) fn next_variant_id(&self) -> VariantId {
        VariantId::new(self.variants.len())
    }

    pub(crate) fn add_variant(&mut self, variant: Variant<'cx>) {
        let id = variant.id;
        assert_eq!(id, self.next_variant_id());
        self.variants.push(variant);
    }

    pub(crate) fn next_assoc_item_id(&self) -> AssocItemId {
        AssocItemId::new(self.assoc_items.len())
    }

    pub(crate) fn add_assoc_item(&mut self, item: AssocItem<'cx>) {
        let id = item.id;
        assert_eq!(id, self.next_assoc_item_id());
        self.assoc_items.push(item);
    }

    pub(crate) fn next_block_id(&self) -> BlockId {
        BlockId::new(self.blocks.len())
    }

    pub(crate) fn add_block(&mut self, block: Block<'cx>) {
        let id = block.id;
        assert_eq!(id, self.next_block_id());
        self.blocks.push(block);
    }

    pub(crate) fn next_type_id(&self) -> TypeId {
        TypeId::new(self.types.len())
    }

    pub(crate) fn add_type(&mut self, ty: Type<'cx>) {
        let id = ty.id;
        assert_eq!(id, self.next_type_id());
        self.types.push(ty);
    }
}

impl<'cx> Index<FileId> for ProgramRepr<'cx> {
    type Output = File<'cx>;

    fn index(&self, id: FileId) -> &Self::Output {
        &self.files[id.index()]
    }
}

impl<'cx> Index<ItemId> for ProgramRepr<'cx> {
    type Output = Item<'cx>;

    fn index(&self, id: ItemId) -> &Self::Output {
        &self.items[id.index()]
    }
}

impl IndexMut<ItemId> for ProgramRepr<'_> {
    fn index_mut(&mut self, id: ItemId) -> &mut Self::Output {
        &mut self.items[id.index()]
    }
}

impl<'cx> Index<SignatureId> for ProgramRepr<'cx> {
    type Output = Signature<'cx>;

    fn index(&self, id: SignatureId) -> &Self::Output {
        &self.signatures[id.index()]
    }
}

impl<'cx> Index<FieldId> for ProgramRepr<'cx> {
    type Output = Field<'cx>;

    fn index(&self, id: FieldId) -> &Self::Output {
        &self.fields[id.index()]
    }
}

impl<'cx> Index<VariantId> for ProgramRepr<'cx> {
    type Output = Variant<'cx>;

    fn index(&self, id: VariantId) -> &Self::Output {
        &self.variants[id.index()]
    }
}

impl<'cx> Index<AssocItemId> for ProgramRepr<'cx> {
    type Output = AssocItem<'cx>;

    fn index(&self, id: AssocItemId) -> &Self::Output {
        &self.assoc_items[id.index()]
    }
}

impl<'cx> Index<BlockId> for ProgramRepr<'cx> {
    type Output = Block<'cx>;

    fn index(&self, id: BlockId) -> &Self::Output {
        &self.blocks[id.index()]
    }
}

impl<'cx> Index<TypeId> for ProgramRepr<'cx> {
    type Output = Type<'cx>;

    fn index(&self, id: TypeId) -> &Self::Output {
        &self.types[id.index()]
    }
}

/// One represented source file.
#[derive(Debug)]
pub struct File<'cx> {
    /// File id in the representation.
    pub id: FileId,
    /// Interned file path.
    pub file_path: FilePath<'cx>,
    /// Top-level represented items in source order.
    pub items: Vec<ItemId>,
}

/// One represented item declaration.
#[derive(Debug)]
pub struct Item<'cx> {
    /// Item id in the representation.
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
        ty: TypeId,
        /// Initializer expression.
        init: Expr,
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
        self_ty: TypeId,
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
        ty: TypeId,
    },
    /// Use item.
    Use,
}

/// Representation-native item generics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generics<'cx> {
    /// Scope where generic parameters and bounds should be resolved.
    pub scope: Option<ScopeId>,
    /// Generic parameters in source order.
    pub params: Vec<GenericParam<'cx>>,
    /// Generic predicates from inline bounds and where-clauses.
    pub predicates: Vec<WherePredicate<'cx>>,
}

/// Representation-native generic parameter declared by an item.
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

/// Representation-native type generic parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParam<'cx> {
    /// Parameter name.
    pub name: Name<'cx>,
    /// Default type, when present.
    pub default: Option<TypeId>,
}

/// Representation-native const generic parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstParam<'cx> {
    /// Parameter name.
    pub name: Name<'cx>,
    /// Parameter type.
    pub ty: TypeId,
}

/// Representation-native type parameter bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeParamBound<'cx> {
    /// Trait bound.
    Trait(TraitBound<'cx>),
    /// Unsupported bound form.
    Unsupported,
}

/// Representation-native trait bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitBound<'cx> {
    /// Trait path.
    pub path: Vec<PathSegment<'cx>>,
}

/// Representation-native generic predicate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WherePredicate<'cx> {
    /// Type bound predicate.
    TypeBound {
        /// Type being constrained.
        subject: TypeId,
        /// Bounds applied to the type.
        bounds: Vec<TypeParamBound<'cx>>,
    },
    /// Unsupported predicate form.
    Unsupported,
}

/// One represented function-like signature.
#[derive(Debug)]
pub struct Signature<'cx> {
    /// Signature id in the representation.
    pub id: SignatureId,
    /// Source signature.
    pub source: SignatureSource,
    /// Signature parameters.
    ///
    /// This is always non-empty. `params[0]` is the output type and has no source pattern.
    /// `params[1..]` are input parameters in source order and have source patterns. Omitted
    /// function returns are represented as unit `()`, which is a tuple type with no element types;
    /// explicitly inferred returns use [`TypeKind::Infer`].
    pub params: Vec<SignatureParam<'cx>>,
}

/// One represented function signature parameter.
#[derive(Debug)]
pub struct SignatureParam<'cx> {
    /// Parameter type.
    pub ty: TypeId,
    /// Source pattern for this parameter.
    ///
    /// This is `None` for the output parameter at `Signature::params[0]` and `Some` for input
    /// parameters at `Signature::params[1..]`.
    pub pat: Option<Pat<'cx>>,
}

/// Pattern representation.
///
/// TODO: Represent patterns natively in `ProgramRepr` instead of keeping the AST pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pat<'cx> {
    /// Original semantic AST pattern.
    pub pat: &'cx ast::Pat<'cx>,
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
    /// Field id in the representation.
    pub id: FieldId,
    /// Field name.
    pub name: Name<'cx>,
    /// Field visibility.
    pub visibility: Visibility<'cx>,
    /// Field type.
    pub ty: TypeId,
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
    /// Variant id in the representation.
    pub id: VariantId,
    /// Definition linked from the current name-resolution data, if available.
    pub def: Option<DefId>,
    /// Variant name.
    pub name: Name<'cx>,
    /// Represented payload fields.
    pub fields: Vec<FieldId>,
    /// Discriminant expression, if present.
    pub discriminant: Option<Expr>,
}

/// One represented associated item declaration.
#[derive(Debug)]
pub struct AssocItem<'cx> {
    /// Associated item id in the representation.
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
        ty: TypeId,
        /// Initializer expression.
        init: Expr,
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
        ty: TypeId,
    },
    /// Trait associated const.
    TraitConst {
        /// Associated const type.
        ty: TypeId,
        /// Optional default expression.
        default: Option<Expr>,
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
        default: Option<TypeId>,
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
#[derive(Debug)]
pub struct Block<'cx> {
    /// Block id in the representation.
    pub id: BlockId,
    /// Original semantic AST block.
    pub block: &'cx ast::Block<'cx>,
    /// Scope containing block-local bindings, if linked from the current name-resolution data.
    pub scope: Option<ScopeId>,
}

/// Expression representation.
///
/// TODO: Represent expressions natively in `ProgramRepr` instead of using this placeholder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Expr;

/// One represented type occurrence.
#[derive(Debug)]
pub struct Type<'cx> {
    /// Type id in the representation.
    pub id: TypeId,
    /// Original semantic AST type, when this type came directly from source syntax.
    ///
    /// This is `None` for synthetic types introduced by representation desugaring.
    pub ty: Option<&'cx ast::Type<'cx>>,
    /// Representation-native source type shape.
    pub kind: TypeKind<'cx>,
    /// Scope used to resolve paths inside this type occurrence.
    pub scope: Option<ScopeId>,
    /// Source role for this type occurrence.
    pub source: TypeSource,
}

/// Representation-native source type shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeKind<'cx> {
    /// Fixed-length array type.
    Array {
        /// Element type.
        elem: TypeId,
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
        elem: TypeId,
        /// Whether the reference is mutable.
        is_mut: bool,
    },
    /// Dynamically sized slice type.
    Slice {
        /// Element type.
        elem: TypeId,
    },
    /// Tuple type.
    Tuple {
        /// Tuple element types.
        elems: Vec<TypeId>,
    },
}

/// Representation-native source path in type position.
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

/// Representation-native qualified self type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QSelf<'cx> {
    /// Self type: `T` in `<T as a::b::Trait>::Assoc`.
    pub self_ty: TypeId,
    /// Trait path segments: `a::b::Trait` in `<T as a::b::Trait>::Assoc`.
    ///
    /// This is empty when the source used `<T>::Assoc` without an explicit trait path.
    pub trait_path: Vec<PathSegment<'cx>>,
}

/// One representation-native type path segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSegment<'cx> {
    /// Segment name.
    pub name: Name<'cx>,
    /// Generic arguments on this segment.
    pub args: Vec<GenericArg<'cx>>,
}

/// Representation-native generic argument supplied at a use site.
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
        ty: TypeId,
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
    /// Length is still a source expression; expression lowering is a future representation slice.
    Expr,
}

/// Representation-native const argument shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstArg<'cx> {
    /// Literal const argument.
    Lit(Lit<'cx>),
    /// Path const argument.
    Path(Path<'cx>),
    /// Const expression that has not been lowered into representation-native expression shape.
    Expr,
}

/// Representation-native literal shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lit<'cx> {
    /// Integer literal stored as normalized base-10 digits.
    Int(InternedStr<'cx>),
    /// Floating-point literal stored as normalized base-10 digits.
    Float(InternedStr<'cx>),
    /// Boolean literal.
    Bool(bool),
}

/// Source role for a represented type occurrence.
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
    /// Nested type inside another represented type occurrence, for example `T` in `Vec<T>`.
    Nested,
}
