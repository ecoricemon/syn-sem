/// Stable kind tag for semantic AST node types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AstNodeKind {
    /// Parsed source file node.
    File,
    /// Braced source block node.
    Block,
    /// Free item wrapper node.
    Item,
    /// Free const item payload node.
    ItemConst,
    /// Enum item payload node.
    ItemEnum,
    /// Free function item payload node.
    ItemFn,
    /// Impl item payload node.
    ItemImpl,
    /// Module item payload node.
    ItemMod,
    /// Struct item payload node.
    ItemStruct,
    /// Trait item payload node.
    ItemTrait,
    /// Type alias item payload node.
    ItemType,
    /// Use item payload node.
    ItemUse,
    /// Identifier binding pattern payload node.
    PatIdent,
    /// Type generic parameter node.
    TypeParam,
    /// Const generic parameter node.
    ConstParam,
    /// Struct or union field node.
    Field,
    /// Enum variant node.
    Variant,
    /// Enum variant field node.
    VariantField,
    /// Impl associated item wrapper node.
    ImplItem,
    /// Impl associated const payload node.
    ImplItemConst,
    /// Impl associated function payload node.
    ImplItemFn,
    /// Impl associated type payload node.
    ImplItemType,
    /// Trait associated item wrapper node.
    TraitItem,
    /// Trait associated const payload node.
    TraitItemConst,
    /// Trait associated function payload node.
    TraitItemFn,
    /// Trait associated type payload node.
    TraitItemType,
}

/// Semantic AST node type with an explicit kind tag.
pub trait AstNode {
    /// Stable kind tag for this AST node type.
    const KIND: AstNodeKind;
}
