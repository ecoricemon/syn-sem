use crate::{
    Block, Expr, Field, FromSyn, Generics, Ident, InputDesc, Pat, PatIdent, PatType, Path, Span,
    SyntaxCx, Type, Variant, Visibility,
};
use std::iter;
use syn_sem_common::FilePath;
use syn_sem_macros::CheckDropless;

/// A top-level or block-level Rust item supported by the semantic AST.
///
/// Examples include `const X: i32 = 1;`, `fn f() {}`, `struct S;`, and `use a::b;`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum Item<'cx> {
    /// Constant item.
    Const(ItemConst<'cx>),
    /// Enum item.
    Enum(ItemEnum<'cx>),
    /// Function item.
    Fn(ItemFn<'cx>),
    /// Implementation block.
    Impl(ItemImpl<'cx>),
    /// Module item.
    Mod(ItemMod<'cx>),
    /// Struct item.
    Struct(ItemStruct<'cx>),
    /// Trait item.
    Trait(ItemTrait<'cx>),
    /// Type alias item.
    Type(ItemType<'cx>),
    /// Use item.
    Use(ItemUse<'cx>),
}

impl<'cx> FromSyn<'cx, syn::Item> for Item<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::Item>) -> Self {
        match desc.input {
            syn::Item::Const(v) => Item::Const(ItemConst::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Item::Enum(v) => Item::Enum(ItemEnum::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Item::Fn(v) => Item::Fn(ItemFn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Item::Impl(v) => Item::Impl(ItemImpl::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Item::Mod(v) => Item::Mod(ItemMod::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Item::Struct(v) => Item::Struct(ItemStruct::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Item::Trait(v) => Item::Trait(ItemTrait::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Item::Type(v) => Item::Type(ItemType::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::Item::Use(v) => Item::Use(ItemUse::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            _ => todo!(),
        }
    }
}

/// A free constant item.
///
/// For example, `pub const N: usize = 4;`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ItemConst<'cx> {
    /// Item visibility.
    pub vis: Visibility<'cx>,
    /// Constant name.
    pub ident: Ident<'cx>,
    /// Constant type.
    pub ty: &'cx Type<'cx>,
    /// Initializer expression.
    pub init: &'cx Expr<'cx>,
    /// Source span of the item.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ItemConst> for ItemConst<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::ItemConst>) -> Self {
        Self {
            vis: Visibility::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.vis,
                },
            ),
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            ty: scx.alloc(Type::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ty,
                },
            )),
            init: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.expr,
                },
            )),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// An enum item.
///
/// For example, `enum Option<T> { Some(T), None }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ItemEnum<'cx> {
    /// Item visibility.
    pub vis: Visibility<'cx>,
    /// Enum name.
    pub ident: Ident<'cx>,
    /// Generic parameters and where-clause.
    pub generics: Generics<'cx>,
    /// Enum variants.
    pub variants: &'cx [Variant<'cx>],
    /// Source span of the item.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ItemEnum> for ItemEnum<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::ItemEnum>) -> Self {
        Self {
            vis: Visibility::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.vis,
                },
            ),
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            generics: Generics::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.generics,
                },
            ),
            variants: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.variants,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A free function item.
///
/// For example, `pub fn add(a: i32, b: i32) -> i32 { a + b }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ItemFn<'cx> {
    /// Item visibility.
    pub vis: Visibility<'cx>,
    /// Generic parameters and where-clause.
    pub generics: Generics<'cx>,
    /// Function signature.
    pub sig: Signature<'cx>,
    /// Function body.
    pub block: Block<'cx>,
    /// Source span of the item.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ItemFn> for ItemFn<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::ItemFn>) -> Self {
        Self {
            vis: Visibility::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.vis,
                },
            ),
            generics: Generics::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.sig.generics,
                },
            ),
            sig: Signature::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.sig,
                },
            ),
            block: Block::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.block,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// An implementation block.
///
/// Examples include `impl S { fn new() {} }` and `impl Trait for S {}`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ItemImpl<'cx> {
    /// Generic parameters and where-clause.
    pub generics: Generics<'cx>,
    /// Implemented trait path, if this is a trait impl.
    pub trait_: Option<Path<'cx>>,
    /// Implementing self type.
    pub self_ty: &'cx Type<'cx>,
    /// Items inside the impl block.
    pub items: &'cx [ImplItem<'cx>],
    /// Source span of the impl block.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ItemImpl> for ItemImpl<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::ItemImpl>) -> Self {
        Self {
            generics: Generics::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.generics,
                },
            ),
            trait_: desc.input.trait_.as_ref().map(|(_, path, _)| {
                Path::from_syn(
                    scx,
                    InputDesc {
                        file_path: desc.file_path,
                        input: path,
                    },
                )
            }),
            self_ty: scx.alloc(Type::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.self_ty,
                },
            )),
            items: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.items,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// An item declared inside an implementation block.
///
/// Examples include `const N: usize = 1;`, `fn f(&self) {}`, and `type Output = T;`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum ImplItem<'cx> {
    /// Associated const.
    Const(ImplItemConst<'cx>),
    /// Associated function.
    Fn(ImplItemFn<'cx>),
    /// Associated type.
    Type(ImplItemType<'cx>),
}

impl<'cx> FromSyn<'cx, syn::ImplItem> for ImplItem<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::ImplItem>) -> Self {
        match desc.input {
            syn::ImplItem::Const(v) => Self::Const(ImplItemConst::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::ImplItem::Fn(v) => Self::Fn(ImplItemFn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::ImplItem::Type(v) => Self::Type(ImplItemType::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            _ => todo!(),
        }
    }
}

/// An associated const inside an implementation block.
///
/// For example, `const N: usize = 1;` in `impl S`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ImplItemConst<'cx> {
    /// Associated const name.
    pub ident: Ident<'cx>,
    /// Generic parameters and where-clause.
    pub generics: Generics<'cx>,
    /// Associated const type.
    pub ty: &'cx Type<'cx>,
    /// Initializer expression.
    pub init: &'cx Expr<'cx>,
    /// Source span of the associated item.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ImplItemConst> for ImplItemConst<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::ImplItemConst>) -> Self {
        Self {
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            generics: Generics::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.generics,
                },
            ),
            ty: scx.alloc(Type::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ty,
                },
            )),
            init: scx.alloc(Expr::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.expr,
                },
            )),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// An associated function inside an implementation block.
///
/// For example, `fn new() -> Self { Self }` in `impl S`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ImplItemFn<'cx> {
    /// Function signature.
    pub sig: Signature<'cx>,
    /// Function body.
    pub block: Block<'cx>,
    /// Source span of the associated item.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ImplItemFn> for ImplItemFn<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::ImplItemFn>) -> Self {
        Self {
            sig: Signature::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.sig,
                },
            ),
            block: Block::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.block,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// An associated type definition inside an implementation block.
///
/// For example, `type Item = T;` in `impl Iterator for S`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ImplItemType<'cx> {
    /// Associated type name.
    pub ident: Ident<'cx>,
    /// Generic parameters and where-clause.
    pub generics: Generics<'cx>,
    /// Assigned type.
    pub ty: &'cx Type<'cx>,
    /// Source span of the associated item.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ImplItemType> for ImplItemType<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::ImplItemType>) -> Self {
        Self {
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            generics: Generics::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.generics,
                },
            ),
            ty: scx.alloc(Type::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ty,
                },
            )),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A module item.
///
/// Examples include inline `mod m { fn f() {} }` and external `mod m;`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ItemMod<'cx> {
    /// Item visibility.
    pub vis: Visibility<'cx>,
    /// Module name.
    pub ident: Ident<'cx>,
    /// Inline module items, if present.
    pub items: Option<&'cx [Item<'cx>]>,
    /// Source span of the item.
    pub span: Span<'cx>,

    /// Whether the module contains its items inline.
    pub is_inline: bool,
}

impl<'cx> FromSyn<'cx, syn::ItemMod> for ItemMod<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::ItemMod>) -> Self {
        Self {
            vis: Visibility::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.vis,
                },
            ),
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            items: desc.input.content.as_ref().map(|(_, items)| {
                <&'cx [Item<'cx>]>::from_syn(
                    scx,
                    InputDesc {
                        file_path: desc.file_path,
                        input: items,
                    },
                )
            }),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
            is_inline: desc.input.content.is_some(),
        }
    }
}

/// A struct item.
///
/// Examples include `struct S;`, `struct S(T);`, and `struct S { x: i32 }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ItemStruct<'cx> {
    /// Item visibility.
    pub vis: Visibility<'cx>,
    /// Struct name.
    pub ident: Ident<'cx>,
    /// Generic parameters and where-clause.
    pub generics: Generics<'cx>,
    /// Struct fields.
    pub fields: &'cx [Field<'cx>],
    /// Source span of the item.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ItemStruct> for ItemStruct<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::ItemStruct>) -> Self {
        Self {
            vis: Visibility::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.vis,
                },
            ),
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            generics: Generics::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.generics,
                },
            ),
            fields: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.fields,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A type alias item.
///
/// For example, `type Bytes = Vec<u8>;`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ItemType<'cx> {
    /// Item visibility.
    pub vis: Visibility<'cx>,
    /// Type alias name.
    pub ident: Ident<'cx>,
    /// Generic parameters and where-clause.
    pub generics: Generics<'cx>,
    /// Aliased type.
    pub ty: &'cx Type<'cx>,
    /// Source span of the item.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ItemType> for ItemType<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::ItemType>) -> Self {
        Self {
            vis: Visibility::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.vis,
                },
            ),
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            generics: Generics::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.generics,
                },
            ),
            ty: scx.alloc(Type::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ty,
                },
            )),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A use item.
///
/// Examples include `use std::fmt;`, `use a::b as c;`, and `use a::{b, c};`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ItemUse<'cx> {
    /// Item visibility.
    pub vis: Visibility<'cx>,
    /// Imported use tree.
    pub tree: UseTree<'cx>,
    /// Source span of the item.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ItemUse> for ItemUse<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::ItemUse>) -> Self {
        Self {
            vis: Visibility::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.vis,
                },
            ),
            tree: UseTree::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.tree,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A tree inside a use item.
///
/// Examples include `a::b`, `a as b`, `*`, and `{a, b}`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum UseTree<'cx> {
    /// Nested use path.
    Path(UsePath<'cx>),
    /// Imported name.
    Name(UseName<'cx>),
    /// Renamed import.
    Rename(UseRename<'cx>),
    /// Glob import.
    Glob(Span<'cx>),
    /// Grouped imports.
    Group(UseGroup<'cx>),
}

impl<'cx> FromSyn<'cx, syn::UseTree> for UseTree<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::UseTree>) -> Self {
        match desc.input {
            syn::UseTree::Path(v) => Self::Path(UsePath::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::UseTree::Name(v) => Self::Name(UseName::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::UseTree::Rename(v) => Self::Rename(UseRename::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::UseTree::Glob(v) => Self::Glob(Span::from_locatable(scx, desc.file_path, v)),
            syn::UseTree::Group(v) => Self::Group(UseGroup::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
        }
    }
}

/// A path node inside a use tree.
///
/// For example, `std::fmt` is represented as nested `UsePath` nodes ending in a name.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct UsePath<'cx> {
    /// Path segment name.
    pub ident: Ident<'cx>,
    /// Nested use tree after this segment.
    pub tree: &'cx UseTree<'cx>,
    /// Source span of the use path.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::UsePath> for UsePath<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::UsePath>) -> Self {
        Self {
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            tree: scx.alloc(UseTree::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.tree,
                },
            )),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A terminal name inside a use tree.
///
/// For example, `fmt` in `use std::fmt;`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct UseName<'cx> {
    /// Imported name.
    pub ident: Ident<'cx>,
    /// Source span of the name.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::UseName> for UseName<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::UseName>) -> Self {
        Self {
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A renamed import inside a use tree.
///
/// For example, `fmt as format` in `use std::fmt as format;`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct UseRename<'cx> {
    /// Original imported name.
    pub ident: Ident<'cx>,
    /// Local renamed name.
    pub rename: Ident<'cx>,
    /// Source span of the rename.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::UseRename> for UseRename<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::UseRename>) -> Self {
        Self {
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            rename: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.rename,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A grouped import inside a use tree.
///
/// For example, `{fmt, io}` in `use std::{fmt, io};`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct UseGroup<'cx> {
    /// Grouped use trees.
    pub items: &'cx [UseTree<'cx>],
    /// Source span of the group.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::UseGroup> for UseGroup<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::UseGroup>) -> Self {
        Self {
            items: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.items,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A trait item.
///
/// For example, `trait Iterator { type Item; fn next(&mut self) -> Option<Self::Item>; }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct ItemTrait<'cx> {
    /// Item visibility.
    pub vis: Visibility<'cx>,
    /// Trait name.
    pub ident: Ident<'cx>,
    /// Generic parameters and where-clause.
    pub generics: Generics<'cx>,
    /// Items declared in the trait.
    pub items: &'cx [TraitItem<'cx>],
    /// Source span of the item.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::ItemTrait> for ItemTrait<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::ItemTrait>) -> Self {
        Self {
            vis: Visibility::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.vis,
                },
            ),
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            generics: Generics::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.generics,
                },
            ),
            items: FromSyn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.items,
                },
            ),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// An item declared inside a trait.
///
/// Examples include associated consts, associated functions, and associated types.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub enum TraitItem<'cx> {
    /// Associated const declaration.
    Const(TraitItemConst<'cx>),
    /// Associated function declaration.
    Fn(TraitItemFn<'cx>),
    /// Associated type declaration.
    Type(TraitItemType<'cx>),
}

impl<'cx> FromSyn<'cx, syn::TraitItem> for TraitItem<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::TraitItem>) -> Self {
        match desc.input {
            syn::TraitItem::Const(v) => Self::Const(TraitItemConst::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::TraitItem::Fn(v) => Self::Fn(TraitItemFn::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            syn::TraitItem::Type(v) => Self::Type(TraitItemType::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            )),
            _ => todo!(),
        }
    }
}

/// An associated const declared in a trait.
///
/// For example, `const N: usize;` or `const N: usize = 1;`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct TraitItemConst<'cx> {
    /// Associated const name.
    pub ident: Ident<'cx>,
    /// Generic parameters and where-clause.
    pub generics: Generics<'cx>,
    /// Associated const type.
    pub ty: &'cx Type<'cx>,
    /// Optional default expression.
    pub default: Option<&'cx Expr<'cx>>,
    /// Source span of the associated item.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::TraitItemConst> for TraitItemConst<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::TraitItemConst>) -> Self {
        Self {
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            generics: Generics::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.generics,
                },
            ),
            ty: scx.alloc(Type::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ty,
                },
            )),
            default: desc.input.default.as_ref().map(|(_, expr)| {
                scx.alloc(Expr::from_syn(
                    scx,
                    InputDesc {
                        file_path: desc.file_path,
                        input: expr,
                    },
                ))
            }),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// An associated function declared in a trait.
///
/// For example, `fn next(&mut self) -> Option<Self::Item>;`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct TraitItemFn<'cx> {
    /// Function signature.
    pub sig: Signature<'cx>,
    /// Optional default body.
    pub default: Option<Block<'cx>>,
    /// Source span of the associated item.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::TraitItemFn> for TraitItemFn<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::TraitItemFn>) -> Self {
        Self {
            sig: Signature::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.sig,
                },
            ),
            default: desc.input.default.as_ref().map(|block| {
                Block::from_syn(
                    scx,
                    InputDesc {
                        file_path: desc.file_path,
                        input: block,
                    },
                )
            }),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// An associated type declared in a trait.
///
/// For example, `type Item;` or `type Item = u8;`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct TraitItemType<'cx> {
    /// Associated type name.
    pub ident: Ident<'cx>,
    /// Generic parameters and where-clause.
    pub generics: Generics<'cx>,
    /// Optional default type.
    pub default: Option<&'cx Type<'cx>>,
    /// Source span of the associated item.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::TraitItemType> for TraitItemType<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::TraitItemType>) -> Self {
        Self {
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            generics: Generics::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.generics,
                },
            ),
            default: desc.input.default.as_ref().map(|(_, ty)| {
                scx.alloc(Type::from_syn(
                    scx,
                    InputDesc {
                        file_path: desc.file_path,
                        input: ty,
                    },
                ))
            }),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// A function-like signature.
///
/// For example, `fn f<T>(x: T) -> T` stores the name, generics, and parameters.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Signature<'cx> {
    /// Function name.
    pub ident: Ident<'cx>,
    /// Generic parameters and where-clause.
    pub generics: Generics<'cx>,
    /// Return parameter followed by input parameters.
    pub params: &'cx [Parameter<'cx>],
    /// Source span of the signature.
    pub span: Span<'cx>,
}

impl<'cx> FromSyn<'cx, syn::Signature> for Signature<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::Signature>) -> Self {
        let output =
            Parameter::from_return_type(scx, desc.file_path, &desc.input.output, ParameterCx::Fn);
        let output = iter::once(output);
        let inputs = desc.input.inputs.iter().map(|arg| {
            Parameter::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: arg,
                },
            )
        });
        let mut params = output.chain(inputs);
        let len = desc.input.inputs.len() + 1;

        Self {
            ident: Ident::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.ident,
                },
            ),
            generics: Generics::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: &desc.input.generics,
                },
            ),
            params: scx.alloc_slice(len, |_| params.next().unwrap()),
            span: Span::from_locatable(scx, desc.file_path, desc.input),
        }
    }
}

/// One function parameter, including the synthesized return parameter at index `0`.
///
/// For example, `x: i32` in `fn f(x: i32)`, or the return type in `fn f() -> i32`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CheckDropless)]
pub struct Parameter<'cx> {
    /// Pattern and type for the parameter.
    pub pat: PatType<'cx>,
    /// Source span of the parameter.
    pub span: Span<'cx>,
}

impl<'cx> Parameter<'cx> {
    /// Creates a parameter with the ident `0`.
    pub fn from_return_type(
        scx: &'cx SyntaxCx<'cx>,
        file_path: FilePath<'cx>,
        ret_ty: &'cx syn::ReturnType,
        parameter_cx: ParameterCx,
    ) -> Self {
        const IDENT: u32 = 0;

        let span = Span::from_locatable(scx, file_path, ret_ty);
        let ty = match ret_ty {
            syn::ReturnType::Default => match parameter_cx {
                ParameterCx::Fn => Type::unit(span),
                ParameterCx::Closure => Type::Infer(span),
            },
            syn::ReturnType::Type(_, ty) => Type::from_syn(
                scx,
                InputDesc {
                    file_path,
                    input: ty,
                },
            ),
        };
        let pat_ident = Pat::Ident(PatIdent::from_number(scx, IDENT, Span::empty()));
        let pat = PatType {
            pat: scx.alloc(pat_ident),
            ty,
            span,
        };
        Self { pat, span }
    }
}

impl<'cx> FromSyn<'cx, syn::FnArg> for Parameter<'cx> {
    fn from_syn(scx: &'cx SyntaxCx<'cx>, desc: InputDesc<'cx, syn::FnArg>) -> Self {
        let span = Span::from_locatable(scx, desc.file_path, desc.input);
        let pat = match desc.input {
            syn::FnArg::Receiver(v) => PatType::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            ),
            syn::FnArg::Typed(v) => PatType::from_syn(
                scx,
                InputDesc {
                    file_path: desc.file_path,
                    input: v,
                },
            ),
        };
        Self { pat, span }
    }
}

/// Context for constructing the synthesized return parameter.
///
/// For example, functions default to `()` while closures can default to an inferred type.
#[derive(PartialEq, Eq)]
pub enum ParameterCx {
    /// Function return parameter context.
    Fn,
    /// Closure return parameter context.
    Closure,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;
    use crate::GenericParam;

    #[test]
    fn item_struct() {
        // Proves structs preserve visibility, name, fields, tuple field names, and generics.
        type T = syn::ItemStruct;
        type U<'a> = ItemStruct<'a>;
        let ccx = syn_sem_common::CommonCx::new();
        let scx = SyntaxCx::new(&ccx);

        // Empty struct
        let st = parse::<T, U>(&scx, "struct A;");
        assert_eq!(&*st.ident.inner, "A");
        assert!(st.fields.is_empty());

        // Tuple struct with zero, one, and two fields.
        let st = parse::<T, U>(&scx, "struct A();");
        assert!(st.fields.is_empty());
        let st = parse::<T, U>(&scx, "struct A(B);");
        assert_eq!(st.fields.len(), 1);
        let Type::Path(ty) = &st.fields[0].ty else {
            panic!()
        };
        assert_eq!(&**ty.path.get_ident().unwrap(), "B");
        let st = parse::<T, U>(&scx, "struct A(B, C);");
        assert_eq!(st.fields.len(), 2);
        assert_eq!(&*st.fields[0].ident, "0");
        assert_eq!(&*st.fields[1].ident, "1");

        // Struct with zero, one, and two fields.
        let st = parse::<T, U>(&scx, "struct A{}");
        assert!(st.fields.is_empty());
        let st = parse::<T, U>(&scx, "struct A{ f1: B }");
        assert_eq!(st.fields.len(), 1);
        assert_eq!(&*st.fields[0].ident, "f1");
        let Type::Path(ty) = &st.fields[0].ty else {
            panic!()
        };
        assert_eq!(&**ty.path.get_ident().unwrap(), "B");
        let st = parse::<T, U>(&scx, "struct A{ f1: B, f2: C }");
        assert_eq!(st.fields.len(), 2);
        let Type::Path(ty) = &st.fields[1].ty else {
            panic!()
        };
        assert_eq!(&**ty.path.get_ident().unwrap(), "C");

        let st = parse::<T, U>(&scx, "struct A<T>{ f: T }");
        assert_eq!(st.generics.params.len(), 1);
        assert!(matches!(st.generics.params[0], GenericParam::Type(_)));
    }

    #[test]
    fn item_enum() {
        // Proves enums preserve name, variants, and generic params.
        type T = syn::ItemEnum;
        type U<'a> = ItemEnum<'a>;
        let ccx = syn_sem_common::CommonCx::new();
        let scx = SyntaxCx::new(&ccx);

        let item_enum = parse::<T, U>(&scx, "enum E<T> { A(T) }");
        assert_eq!(&*item_enum.ident, "E");
        assert_eq!(item_enum.generics.params.len(), 1);
        assert!(matches!(
            item_enum.generics.params[0],
            GenericParam::Type(_)
        ));
    }

    #[test]
    fn item_fn() {
        // Proves free functions preserve signature generics and parameters.
        type T = syn::ItemFn;
        type U<'a> = ItemFn<'a>;
        let ccx = syn_sem_common::CommonCx::new();
        let scx = SyntaxCx::new(&ccx);

        let item_fn = parse::<T, U>(&scx, "fn f<T>(value: T) -> T { value }");
        assert_eq!(&*item_fn.sig.ident, "f");
        assert_eq!(item_fn.generics.params.len(), 1);
        assert_eq!(item_fn.sig.generics.params.len(), 1);
        assert!(matches!(
            item_fn.sig.generics.params[0],
            GenericParam::Type(_)
        ));
    }

    #[test]
    fn item_mod() {
        // Proves modules distinguish inline modules from file-backed declarations.
        type T = syn::ItemMod;
        type U<'a> = ItemMod<'a>;
        let ccx = syn_sem_common::CommonCx::new();
        let scx = SyntaxCx::new(&ccx);

        let item_mod = parse::<T, U>(&scx, "mod a;");
        assert_eq!(&*item_mod.ident, "a");
        assert!(item_mod.items.is_none());
        assert!(!item_mod.is_inline);

        let item_mod = parse::<T, U>(&scx, "pub mod a { const N: usize = 1; struct S; }");
        assert!(matches!(item_mod.vis, Visibility::Public(..)));
        assert!(item_mod.is_inline);
        let items = item_mod.items.unwrap();
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0], Item::Const(_)));
        assert!(matches!(items[1], Item::Struct(_)));
    }

    #[test]
    fn item_type() {
        // Proves type aliases preserve visibility, name, target type, and generics.
        type T = syn::ItemType;
        type U<'a> = ItemType<'a>;
        let ccx = syn_sem_common::CommonCx::new();
        let scx = SyntaxCx::new(&ccx);

        let item_type = parse::<T, U>(&scx, "pub type Alias = Target;");
        assert!(matches!(item_type.vis, Visibility::Public(..)));
        assert_eq!(&*item_type.ident, "Alias");
        assert!(matches!(item_type.ty, Type::Path(_)));

        let item_type = parse::<T, U>(&scx, "type Alias<T> = T;");
        assert_eq!(item_type.generics.params.len(), 1);
        assert!(matches!(
            item_type.generics.params[0],
            GenericParam::Type(_)
        ));

        let item_mod = parse::<syn::ItemMod, ItemMod>(&scx, "mod a { type Alias = Target; }");
        let items = item_mod.items.unwrap();
        assert_eq!(items.len(), 1);
        assert!(matches!(items[0], Item::Type(_)));
    }

    #[test]
    fn item_use() {
        // Proves use items preserve all supported use tree forms.
        type T = syn::ItemUse;
        type U<'a> = ItemUse<'a>;
        let ccx = syn_sem_common::CommonCx::new();
        let scx = SyntaxCx::new(&ccx);

        let item_use = parse::<T, U>(&scx, "pub use a;");
        assert!(matches!(item_use.vis, Visibility::Public(..)));
        let UseTree::Name(name) = &item_use.tree else {
            panic!()
        };
        assert_eq!(&*name.ident, "a");

        let item_use = parse::<T, U>(&scx, "use a::b as c;");
        let UseTree::Path(path) = &item_use.tree else {
            panic!()
        };
        assert_eq!(&*path.ident, "a");
        let UseTree::Rename(rename) = path.tree else {
            panic!()
        };
        assert_eq!(&*rename.ident, "b");
        assert_eq!(&*rename.rename, "c");

        let item_use = parse::<T, U>(&scx, "use a::{b, c as d, *};");
        let UseTree::Path(path) = &item_use.tree else {
            panic!()
        };
        assert_eq!(&*path.ident, "a");
        let UseTree::Group(group) = path.tree else {
            panic!()
        };
        assert_eq!(group.items.len(), 3);
        assert!(matches!(group.items[0], UseTree::Name(_)));
        assert!(matches!(group.items[1], UseTree::Rename(_)));
        assert!(matches!(group.items[2], UseTree::Glob(_)));

        let item_mod = parse::<syn::ItemMod, ItemMod>(&scx, "mod a { use b::c; }");
        let items = item_mod.items.unwrap();
        assert_eq!(items.len(), 1);
        assert!(matches!(items[0], Item::Use(_)));
    }

    #[test]
    fn item_impl() {
        // Proves impl blocks preserve generics, trait target, self type, and associated items.
        type T = syn::ItemImpl;
        type U<'a> = ItemImpl<'a>;
        let ccx = syn_sem_common::CommonCx::new();
        let scx = SyntaxCx::new(&ccx);

        let item_impl = parse::<T, U>(&scx, "impl S { const N: usize = 1; fn f(&self) {} }");
        assert!(item_impl.generics.params.is_empty());
        assert!(item_impl.trait_.is_none());
        assert!(matches!(item_impl.self_ty, Type::Path(_)));
        assert_eq!(item_impl.items.len(), 2);

        let ImplItem::Const(item_const) = &item_impl.items[0] else {
            panic!()
        };
        assert_eq!(&*item_const.ident, "N");
        assert!(item_const.generics.params.is_empty());

        let ImplItem::Fn(item_fn) = &item_impl.items[1] else {
            panic!()
        };
        assert_eq!(&*item_fn.sig.ident, "f");
        assert_eq!(item_fn.sig.params.len(), 2);

        let item_impl = parse::<T, U>(&scx, "impl Trait for S { type Assoc = usize; }");
        assert_eq!(
            &**item_impl.trait_.as_ref().unwrap().get_ident().unwrap(),
            "Trait"
        );
        assert!(matches!(item_impl.self_ty, Type::Path(_)));
        assert_eq!(item_impl.items.len(), 1);

        let ImplItem::Type(item_type) = &item_impl.items[0] else {
            panic!()
        };
        assert_eq!(&*item_type.ident, "Assoc");

        let item_impl = parse::<T, U>(&scx, "impl<T> S<T> { const C: usize = 0; type A<U> = U; }");
        assert_eq!(item_impl.generics.params.len(), 1);
        let ImplItem::Const(item_const) = &item_impl.items[0] else {
            panic!()
        };
        assert!(item_const.generics.params.is_empty());
        let ImplItem::Type(item_type) = &item_impl.items[1] else {
            panic!()
        };
        assert_eq!(item_type.generics.params.len(), 1);
    }

    #[test]
    fn item_trait() {
        // Proves traits preserve generics and supported associated item defaults.
        type T = syn::ItemTrait;
        type U<'a> = ItemTrait<'a>;
        let ccx = syn_sem_common::CommonCx::new();
        let scx = SyntaxCx::new(&ccx);

        let item_trait = parse::<T, U>(
            &scx,
            "pub trait Trait {
                const REQUIRED: usize;
                const DEFAULTED: usize = 1;
                type Assoc;
                type DefaultAssoc = usize;
                fn required(&self);
                fn defaulted(&self) {}
            }",
        );

        assert!(matches!(item_trait.vis, Visibility::Public(..)));
        assert_eq!(&*item_trait.ident, "Trait");
        assert!(item_trait.generics.params.is_empty());
        assert_eq!(item_trait.items.len(), 6);

        let TraitItem::Const(item_const) = &item_trait.items[0] else {
            panic!()
        };
        assert_eq!(&*item_const.ident, "REQUIRED");
        assert!(item_const.generics.params.is_empty());
        assert!(item_const.default.is_none());

        let TraitItem::Const(item_const) = &item_trait.items[1] else {
            panic!()
        };
        assert_eq!(&*item_const.ident, "DEFAULTED");
        assert!(item_const.default.is_some());

        let TraitItem::Type(item_type) = &item_trait.items[2] else {
            panic!()
        };
        assert_eq!(&*item_type.ident, "Assoc");
        assert!(item_type.default.is_none());

        let TraitItem::Type(item_type) = &item_trait.items[3] else {
            panic!()
        };
        assert_eq!(&*item_type.ident, "DefaultAssoc");
        assert!(item_type.default.is_some());

        let TraitItem::Fn(item_fn) = &item_trait.items[4] else {
            panic!()
        };
        assert_eq!(&*item_fn.sig.ident, "required");
        assert!(item_fn.default.is_none());

        let TraitItem::Fn(item_fn) = &item_trait.items[5] else {
            panic!()
        };
        assert_eq!(&*item_fn.sig.ident, "defaulted");
        assert!(item_fn.default.is_some());

        let item_trait = parse::<T, U>(
            &scx,
            "trait Trait<T> {
                const C: usize;
                type Assoc<U>;
                fn f<U>(&self, value: U);
            }",
        );
        assert_eq!(item_trait.generics.params.len(), 1);
        let TraitItem::Const(item_const) = &item_trait.items[0] else {
            panic!()
        };
        assert!(item_const.generics.params.is_empty());
        let TraitItem::Type(item_type) = &item_trait.items[1] else {
            panic!()
        };
        assert_eq!(item_type.generics.params.len(), 1);
        let TraitItem::Fn(item_fn) = &item_trait.items[2] else {
            panic!()
        };
        assert_eq!(item_fn.sig.generics.params.len(), 1);
    }
}
