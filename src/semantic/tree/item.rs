use super::{
    format,
    format::PrintFilter,
    node::NodeIndex,
    ty::{TypeId, OWNED_TYPE_CREATOR},
    PathId,
};
use crate::{
    etc::util,
    syntax::common::{AttributeHelper, IdentifySyn, SynId},
    TriOption, Which2,
};
use smallvec::SmallVec;
use std::{
    cmp, fmt, mem, ops,
    path::{Path as StdPath, PathBuf},
    ptr::NonNull,
};

#[derive(Default, Clone, PartialEq)]
pub enum PrivItem {
    Block(Block),
    Const(Const),
    Enum(Enum),
    Field(Field),
    Fn(Fn),
    Local(Local),
    Mod(Mod),
    Struct(Struct),
    Trait(Trait),
    TypeAlias(TypeAlias),
    Use(Use),
    Variant(Variant),
    RawConst(RawConst),
    RawEnum(RawEnum),
    RawField(RawField),
    RawFn(RawFn),
    RawLocal(RawLocal),
    RawMod(RawMod),
    RawStruct(RawStruct),
    RawTrait(RawTrait),
    RawTypeAlias(RawTypeAlias),
    RawUse(RawUse),
    RawVariant(RawVariant),
    #[default]
    None,
}

impl PrivItem {
    pub(crate) fn as_block(&self) -> &Block {
        let Self::Block(v) = self else {
            panic!("expected `PrivItem::Block`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_use(&self) -> &Use {
        let Self::Use(v) = self else {
            panic!("expected `PrivItem::Use`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_mut_mod(&mut self) -> &mut Mod {
        let Self::Mod(v) = self else {
            panic!("expected `PrivItem::Mod`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_mut_type_alias(&mut self) -> &mut TypeAlias {
        let Self::TypeAlias(v) = self else {
            panic!("expected `PrivItem::TypeAlias`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_mut_use(&mut self) -> &mut Use {
        let Self::Use(v) = self else {
            panic!("expected `PrivItem::Use`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_raw_const(&self) -> &RawConst {
        let Self::RawConst(v) = self else {
            panic!("expected `PrivItem::RawConst`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_raw_enum(&self) -> &RawEnum {
        let Self::RawEnum(v) = self else {
            panic!("expected `PrivItem::RawEnum`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_raw_field(&self) -> &RawField {
        let Self::RawField(v) = self else {
            panic!("expected `PrivItem::RawField`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_raw_fn(&self) -> &RawFn {
        let Self::RawFn(v) = self else {
            panic!("expected `PrivItem::RawFn`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_raw_local(&self) -> &RawLocal {
        let Self::RawLocal(v) = self else {
            panic!("expected `PrivItem::RawLocal`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_raw_mod(&self) -> &RawMod {
        let Self::RawMod(v) = self else {
            panic!("expected `PrivItem::RawMod`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_raw_struct(&self) -> &RawStruct {
        let Self::RawStruct(v) = self else {
            panic!("expected `PrivItem::RawStruct`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_raw_trait(&self) -> &RawTrait {
        let Self::RawTrait(v) = self else {
            panic!("expected `PrivItem::RawTrait`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_raw_type_alias(&self) -> &RawTypeAlias {
        let Self::RawTypeAlias(v) = self else {
            panic!("expected `PrivItem::RawTypeAlias`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_raw_use(&self) -> &RawUse {
        let Self::RawUse(v) = self else {
            panic!("expected `PrivItem::RawUse`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_raw_variant(&self) -> &RawVariant {
        let Self::RawVariant(v) = self else {
            panic!("expected `PrivItem::RawVariant`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_mut_raw_const(&mut self) -> &mut RawConst {
        let Self::RawConst(v) = self else {
            panic!("expected `PrivItem::RawConst`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_mut_raw_enum(&mut self) -> &mut RawEnum {
        let Self::RawEnum(v) = self else {
            panic!("expected `PrivItem::RawEnum`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_mut_raw_field(&mut self) -> &mut RawField {
        let Self::RawField(v) = self else {
            panic!("expected `PrivItem::RawField`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_mut_raw_fn(&mut self) -> &mut RawFn {
        let Self::RawFn(v) = self else {
            panic!("expected `PrivItem::RawFn`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_mut_raw_mod(&mut self) -> &mut RawMod {
        let Self::RawMod(v) = self else {
            panic!("expected `PrivItem::RawMod`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_mut_raw_struct(&mut self) -> &mut RawStruct {
        let Self::RawStruct(v) = self else {
            panic!("expected `PrivItem::RawStruct`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_mut_raw_type_alias(&mut self) -> &mut RawTypeAlias {
        let Self::RawTypeAlias(v) = self else {
            panic!("expected `PrivItem::RawTypeAlias`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_mut_raw_use(&mut self) -> &mut RawUse {
        let Self::RawUse(v) = self else {
            panic!("expected `PrivItem::RawUse`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn as_mut_raw_variant(&mut self) -> &mut RawVariant {
        let Self::RawVariant(v) = self else {
            panic!("expected `PrivItem::RawVariant`, but found {self:#?}");
        };
        v
    }

    pub(crate) fn is_raw(&self) -> bool {
        match self {
            Self::Block(_)
            | Self::Const(_)
            | Self::Enum(_)
            | Self::Field(_)
            | Self::Fn(_)
            | Self::Local(_)
            | Self::Mod(_)
            | Self::Struct(_)
            | Self::Trait(_)
            | Self::TypeAlias(_)
            | Self::Use(_)
            | Self::Variant(_)
            | Self::None => false,

            Self::RawConst(_)
            | Self::RawEnum(_)
            | Self::RawField(_)
            | Self::RawFn(_)
            | Self::RawLocal(_)
            | Self::RawMod(_)
            | Self::RawStruct(_)
            | Self::RawTrait(_)
            | Self::RawTypeAlias(_)
            | Self::RawUse(_)
            | Self::RawVariant(_) => true,
        }
    }
}

impl fmt::Debug for PrivItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Block(v) => v.fmt(f),
            Self::Const(v) => v.fmt(f),
            Self::Enum(v) => v.fmt(f),
            Self::Field(v) => v.fmt(f),
            Self::Fn(v) => v.fmt(f),
            Self::Local(v) => v.fmt(f),
            Self::Mod(v) => v.fmt(f),
            Self::Struct(v) => v.fmt(f),
            Self::Trait(v) => v.fmt(f),
            Self::TypeAlias(v) => v.fmt(f),
            Self::Use(v) => v.fmt(f),
            Self::Variant(v) => v.fmt(f),
            Self::RawConst(v) => v.fmt(f),
            Self::RawEnum(v) => v.fmt(f),
            Self::RawField(v) => v.fmt(f),
            Self::RawFn(v) => v.fmt(f),
            Self::RawLocal(v) => v.fmt(f),
            Self::RawMod(v) => v.fmt(f),
            Self::RawStruct(v) => v.fmt(f),
            Self::RawTrait(v) => v.fmt(f),
            Self::RawTypeAlias(v) => v.fmt(f),
            Self::RawUse(v) => v.fmt(f),
            Self::RawVariant(v) => v.fmt(f),
            Self::None => f.write_str("None"),
        }
    }
}

impl format::DebugBriefly for PrivItem {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, filter: &PrintFilter) -> fmt::Result {
        match self {
            Self::Block(v) => v.fmt_briefly(f, filter),
            Self::Const(v) => v.fmt_briefly(f, filter),
            Self::Enum(v) => v.fmt_briefly(f, filter),
            Self::Field(v) => v.fmt_briefly(f, filter),
            Self::Fn(v) => v.fmt_briefly(f, filter),
            Self::Local(v) => v.fmt_briefly(f, filter),
            Self::Mod(v) => v.fmt_briefly(f, filter),
            Self::Struct(v) => v.fmt_briefly(f, filter),
            Self::Trait(v) => v.fmt_briefly(f, filter),
            Self::TypeAlias(v) => v.fmt_briefly(f, filter),
            Self::Use(v) => v.fmt_briefly(f, filter),
            Self::Variant(v) => v.fmt_briefly(f, filter),
            Self::RawConst(v) => v.fmt_briefly(f, filter),
            Self::RawEnum(v) => v.fmt_briefly(f, filter),
            Self::RawField(v) => v.fmt_briefly(f, filter),
            Self::RawFn(v) => v.fmt_briefly(f, filter),
            Self::RawLocal(v) => v.fmt_briefly(f, filter),
            Self::RawMod(v) => v.fmt_briefly(f, filter),
            Self::RawStruct(v) => v.fmt_briefly(f, filter),
            Self::RawTrait(v) => v.fmt_briefly(f, filter),
            Self::RawTypeAlias(v) => v.fmt_briefly(f, filter),
            Self::RawUse(v) => v.fmt_briefly(f, filter),
            Self::RawVariant(v) => v.fmt_briefly(f, filter),
            Self::None => f.write_str("None"),
        }
    }

    fn name(&self) -> &'static str {
        "PrivItem"
    }
}

pub enum PubItem<'a> {
    Block(&'a Block),
    Const(&'a Const),
    Enum(&'a Enum),
    Field(&'a Field),
    Fn(&'a Fn),
    Local(&'a Local),
    Mod(&'a Mod),
    Struct(&'a Struct),
    Trait(&'a Trait),
    TypeAlias(&'a TypeAlias),
    Use(&'a Use),
    Variant(&'a Variant),
}

impl<'a> PubItem<'a> {
    pub(crate) fn new(item: &'a PrivItem) -> Option<Self> {
        match item {
            PrivItem::Block(v) => Some(Self::Block(v)),
            PrivItem::Const(v) => Some(Self::Const(v)),
            PrivItem::Enum(v) => Some(Self::Enum(v)),
            PrivItem::Field(v) => Some(Self::Field(v)),
            PrivItem::Fn(v) => Some(Self::Fn(v)),
            PrivItem::Local(v) => Some(Self::Local(v)),
            PrivItem::Mod(v) => Some(Self::Mod(v)),
            PrivItem::Struct(v) => Some(Self::Struct(v)),
            PrivItem::Trait(v) => Some(Self::Trait(v)),
            PrivItem::TypeAlias(v) => Some(Self::TypeAlias(v)),
            PrivItem::Use(v) => Some(Self::Use(v)),
            PrivItem::Variant(v) => Some(Self::Variant(v)),
            PrivItem::RawConst(_)
            | PrivItem::RawEnum(_)
            | PrivItem::RawField(_)
            | PrivItem::RawFn(_)
            | PrivItem::RawLocal(_)
            | PrivItem::RawMod(_)
            | PrivItem::RawStruct(_)
            | PrivItem::RawTrait(_)
            | PrivItem::RawTypeAlias(_)
            | PrivItem::RawUse(_)
            | PrivItem::RawVariant(_)
            | PrivItem::None => None,
        }
    }

    pub fn as_block(&self) -> Option<&Block> {
        if let Self::Block(inner) = self {
            Some(inner)
        } else {
            None
        }
    }

    pub fn as_const(&self) -> Option<&Const> {
        if let Self::Const(inner) = self {
            Some(inner)
        } else {
            None
        }
    }

    pub fn as_field(&self) -> Option<&Field> {
        if let Self::Field(inner) = self {
            Some(inner)
        } else {
            None
        }
    }

    pub fn as_fn(&self) -> Option<&Fn> {
        if let Self::Fn(inner) = self {
            Some(inner)
        } else {
            None
        }
    }

    pub fn as_local(&self) -> Option<&Local> {
        if let Self::Local(inner) = self {
            Some(inner)
        } else {
            None
        }
    }

    pub fn as_mod(&self) -> Option<&Mod> {
        if let Self::Mod(inner) = self {
            Some(inner)
        } else {
            None
        }
    }

    pub fn as_struct(&self) -> Option<&Struct> {
        if let Self::Struct(inner) = self {
            Some(inner)
        } else {
            None
        }
    }

    pub fn as_trait(&self) -> Option<&Trait> {
        if let Self::Trait(inner) = self {
            Some(inner)
        } else {
            None
        }
    }

    pub fn as_type_alias(&self) -> Option<&TypeAlias> {
        if let Self::TypeAlias(inner) = self {
            Some(inner)
        } else {
            None
        }
    }

    pub fn as_use(&self) -> Option<&Use> {
        if let Self::Use(inner) = self {
            Some(inner)
        } else {
            None
        }
    }
}

impl fmt::Debug for PubItem<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Block(v) => v.fmt(f),
            Self::Const(v) => v.fmt(f),
            Self::Enum(v) => v.fmt(f),
            Self::Field(v) => v.fmt(f),
            Self::Fn(v) => v.fmt(f),
            Self::Local(v) => v.fmt(f),
            Self::Mod(v) => v.fmt(f),
            Self::Struct(v) => v.fmt(f),
            Self::Trait(v) => v.fmt(f),
            Self::TypeAlias(v) => v.fmt(f),
            Self::Use(v) => v.fmt(f),
            Self::Variant(v) => v.fmt(f),
        }
    }
}

impl format::DebugBriefly for PubItem<'_> {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, filter: &PrintFilter) -> fmt::Result {
        match self {
            Self::Block(v) => v.fmt_briefly(f, filter),
            Self::Const(v) => v.fmt_briefly(f, filter),
            Self::Enum(v) => v.fmt_briefly(f, filter),
            Self::Field(v) => v.fmt_briefly(f, filter),
            Self::Fn(v) => v.fmt_briefly(f, filter),
            Self::Local(v) => v.fmt_briefly(f, filter),
            Self::Mod(v) => v.fmt_briefly(f, filter),
            Self::Struct(v) => v.fmt_briefly(f, filter),
            Self::Trait(v) => v.fmt_briefly(f, filter),
            Self::TypeAlias(v) => v.fmt_briefly(f, filter),
            Self::Use(v) => v.fmt_briefly(f, filter),
            Self::Variant(v) => v.fmt_briefly(f, filter),
        }
    }

    fn name(&self) -> &'static str {
        "PubItem"
    }
}

impl AttributeHelper for PubItem<'_> {
    fn get_attributes(&self) -> Option<&Vec<syn::Attribute>> {
        match self {
            Self::Block(v) => v.get_attributes(),
            Self::Const(v) => v.get_attributes(),
            Self::Enum(v) => v.get_attributes(),
            Self::Field(v) => v.get_attributes(),
            Self::Fn(v) => v.get_attributes(),
            Self::Local(v) => v.get_attributes(),
            Self::Mod(v) => v.get_attributes(),
            Self::Struct(v) => v.get_attributes(),
            Self::Trait(v) => v.get_attributes(),
            Self::TypeAlias(v) => v.get_attributes(),
            Self::Use(v) => v.get_attributes(),
            Self::Variant(v) => v.get_attributes(),
        }
    }

    fn get_mut_attributes(&mut self) -> Option<&mut Vec<syn::Attribute>> {
        None // Read-only
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Block {
    /// Read-only pointer to a syntax tree node that never changes.
    pub(crate) ptr_block: NonNull<syn::Block>,
}

impl Block {
    pub fn ptr_syn(&self) -> *const syn::Block {
        self.ptr_block.as_ptr().cast_const()
    }

    pub fn as_syn<'o>(&self) -> &'o syn::Block {
        unsafe { self.ptr_block.as_ref() }
    }

    pub fn syn_id(&self) -> SynId {
        self.as_syn().syn_id()
    }
}

impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name).finish()
    }
}

impl format::DebugBriefly for Block {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name).finish()
    }

    fn name(&self) -> &'static str {
        "Block"
    }
}

impl AttributeHelper for Block {
    fn get_attributes(&self) -> Option<&Vec<syn::Attribute>> {
        None // No attributes
    }

    fn get_mut_attributes(&mut self) -> Option<&mut Vec<syn::Attribute>> {
        None // No attributes
    }
}

#[derive(Clone, PartialEq)]
pub enum Const {
    Free {
        /// Read-only pointer to a syntax tree node that never changes.
        ptr_const: NonNull<syn::ItemConst>,
        /// Refer to the [module documentation](self).
        vis_node: NodeIndex,
        /// Type of this item.
        tid: TypeId,
    },
    Inher {
        /// Read-only pointer to a syntax tree node that never changes.
        ptr_const: NonNull<syn::ImplItemConst>,
        /// Refer to the [module documentation](self).
        vis_node: NodeIndex,
        /// Type of this item.
        tid: TypeId,
    },
    TraitDefault {
        /// Read-only pointer to a syntax tree node that never changes.
        ptr_const: NonNull<syn::TraitItemConst>,
        /// Refer to the [module documentation](self).
        vis_node: NodeIndex,
        /// Type of this item.
        tid: TypeId,
    },
    TraitImpl {
        /// Read-only pointer to a syntax tree node that never changes.
        ptr_const: NonNull<syn::ImplItemConst>,
        /// Refer to the [module documentation](self).
        vis_node: NodeIndex,
        /// Type of this item.
        tid: TypeId,
    },
}

impl Const {
    pub fn ptr_syn(&self) -> ConstPtr {
        match self {
            Self::Free { ptr_const, .. } => ConstPtr::Free(ptr_const.as_ptr().cast_const()),
            Self::Inher { ptr_const, .. } => ConstPtr::Inher(ptr_const.as_ptr().cast_const()),
            Self::TraitDefault { ptr_const, .. } => {
                ConstPtr::TraitDefault(ptr_const.as_ptr().cast_const())
            }
            Self::TraitImpl { ptr_const, .. } => {
                ConstPtr::TraitImpl(ptr_const.as_ptr().cast_const())
            }
        }
    }

    pub fn syn_id(&self) -> SynId {
        match self {
            Self::Free { ptr_const, .. } => unsafe { ptr_const.as_ref().syn_id() },
            Self::Inher { ptr_const, .. } => unsafe { ptr_const.as_ref().syn_id() },
            Self::TraitDefault { ptr_const, .. } => unsafe { ptr_const.as_ref().syn_id() },
            Self::TraitImpl { ptr_const, .. } => unsafe { ptr_const.as_ref().syn_id() },
        }
    }

    pub fn syn_type(&self) -> &syn::Type {
        match self {
            Self::Free { ptr_const, .. } => unsafe { &ptr_const.as_ref().ty },
            Self::Inher { ptr_const, .. } => unsafe { &ptr_const.as_ref().ty },
            Self::TraitDefault { ptr_const, .. } => unsafe { &ptr_const.as_ref().ty },
            Self::TraitImpl { ptr_const, .. } => unsafe { &ptr_const.as_ref().ty },
        }
    }

    pub fn syn_expr(&self) -> Option<&syn::Expr> {
        match self {
            Self::Free { ptr_const, .. } => unsafe { Some(&ptr_const.as_ref().expr) },
            Self::Inher { ptr_const, .. } => unsafe { Some(&ptr_const.as_ref().expr) },
            Self::TraitDefault { ptr_const, .. } => unsafe {
                ptr_const.as_ref().default.as_ref().map(|(_, expr)| expr)
            },
            Self::TraitImpl { ptr_const, .. } => unsafe { Some(&ptr_const.as_ref().expr) },
        }
    }

    pub fn vis_node(&self) -> NodeIndex {
        match self {
            Self::Free { vis_node, .. } => *vis_node,
            Self::Inher { vis_node, .. } => *vis_node,
            Self::TraitDefault { vis_node, .. } => *vis_node,
            Self::TraitImpl { vis_node, .. } => *vis_node,
        }
    }

    pub fn type_id(&self) -> TypeId {
        match self {
            Self::Free { tid, .. } => *tid,
            Self::Inher { tid, .. } => *tid,
            Self::TraitDefault { tid, .. } => *tid,
            Self::TraitImpl { tid, .. } => *tid,
        }
    }
}

impl fmt::Debug for Const {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        let vis_node = self.vis_node();
        let tid = self.type_id();

        let mut d = f.debug_struct(name);
        d.field("vis_node", &vis_node);
        if let Some(creator) =
            unsafe { OWNED_TYPE_CREATOR.with(|creator| creator.get().map(|ptr| ptr.as_ref())) }
        {
            d.field("ty", &creator.create_owned_type(tid));
        } else {
            d.field("tid", &tid);
        }
        d.finish()
    }
}

impl format::DebugBriefly for Const {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        let tid = self.type_id();

        let mut d = f.debug_struct(name);
        if let Some(creator) =
            unsafe { OWNED_TYPE_CREATOR.with(|creator| creator.get().map(|ptr| ptr.as_ref())) }
        {
            d.field("ty", &creator.create_owned_type(tid));
        } else {
            d.field("tid", &tid);
        }
        d.finish()
    }

    fn name(&self) -> &'static str {
        "Const"
    }
}

impl AttributeHelper for Const {
    fn get_attributes(&self) -> Option<&Vec<syn::Attribute>> {
        match self {
            Self::Free { ptr_const, .. } => unsafe { Some(&ptr_const.as_ref().attrs) },
            Self::Inher { ptr_const, .. } => unsafe { Some(&ptr_const.as_ref().attrs) },
            Self::TraitDefault { ptr_const, .. } => unsafe { Some(&ptr_const.as_ref().attrs) },
            Self::TraitImpl { ptr_const, .. } => unsafe { Some(&ptr_const.as_ref().attrs) },
        }
    }

    fn get_mut_attributes(&mut self) -> Option<&mut Vec<syn::Attribute>> {
        None // Read only
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum RawConst {
    Free {
        /// Read-only pointer to a syntax tree node that never changes.
        ptr_const: NonNull<syn::ItemConst>,
        /// Refer to the [module documentation](self).
        vis_node: Option<NodeIndex>,
        /// Type of this item.
        tid: Option<TypeId>,
    },
}

impl RawConst {
    pub(crate) fn ptr_syn(&self) -> ConstPtr {
        match self {
            Self::Free { ptr_const, .. } => ConstPtr::Free(ptr_const.as_ptr().cast_const()),
        }
    }

    pub(crate) fn syn_id(&self) -> SynId {
        match self {
            Self::Free { ptr_const, .. } => unsafe { ptr_const.as_ref().syn_id() },
        }
    }

    pub(crate) fn syn_type(&self) -> &syn::Type {
        match self {
            Self::Free { ptr_const, .. } => unsafe { &ptr_const.as_ref().ty },
        }
    }

    pub(crate) fn vis_node(&self) -> Option<NodeIndex> {
        match self {
            Self::Free { vis_node, .. } => *vis_node,
        }
    }

    pub(crate) fn type_id(&self) -> Option<TypeId> {
        match self {
            Self::Free { tid, .. } => *tid,
        }
    }

    pub(crate) fn visibility(&self) -> PathVis {
        let vis = match self {
            Self::Free { ptr_const, .. } => unsafe { &ptr_const.as_ref().vis },
        };
        PathVis::new(vis)
    }
}

impl fmt::Debug for RawConst {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        let vis_node = self.vis_node();
        let tid = self.type_id();

        let mut d = f.debug_struct(name);
        d.field("vis_node", &vis_node);
        if let Some(creator) =
            unsafe { OWNED_TYPE_CREATOR.with(|creator| creator.get().map(|ptr| ptr.as_ref())) }
        {
            d.field("ty", &tid.map(|tid| creator.create_owned_type(tid)));
        } else {
            d.field("tid", &tid);
        }
        d.finish()
    }
}

impl format::DebugBriefly for RawConst {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        let tid = self.type_id();

        let mut d = f.debug_struct(name);
        if let Some(creator) =
            unsafe { OWNED_TYPE_CREATOR.with(|creator| creator.get().map(|ptr| ptr.as_ref())) }
        {
            d.field("ty", &tid.map(|tid| creator.create_owned_type(tid)));
        } else {
            d.field("tid", &tid);
        }
        d.finish()
    }

    fn name(&self) -> &'static str {
        "RawConst"
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Enum {
    /// Read-only pointer to a syntax tree node that never changes.
    pub(crate) ptr_enum: NonNull<syn::ItemEnum>,

    /// Refer to the [module documentation](self).
    pub(crate) vis_node: NodeIndex,

    /// Type of this item.
    pub(crate) tid: TypeId,
}

impl Enum {
    pub fn ptr_syn(&self) -> *const syn::ItemEnum {
        self.ptr_enum.as_ptr().cast_const()
    }

    pub fn as_syn<'o>(&self) -> &'o syn::ItemEnum {
        unsafe { self.ptr_enum.as_ref() }
    }

    pub fn syn_id(&self) -> SynId {
        self.as_syn().syn_id()
    }
}

impl fmt::Debug for Enum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        if let Some(creator) =
            unsafe { OWNED_TYPE_CREATOR.with(|creator| creator.get().map(|ptr| ptr.as_ref())) }
        {
            f.debug_struct(name)
                .field("vis_node", &self.vis_node)
                .field("ty", &creator.create_owned_type(self.tid))
                .finish()
        } else {
            f.debug_struct(name)
                .field("vis_node", &self.vis_node)
                .field("tid", &self.tid)
                .finish()
        }
    }
}

impl format::DebugBriefly for Enum {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        if let Some(creator) =
            unsafe { OWNED_TYPE_CREATOR.with(|creator| creator.get().map(|ptr| ptr.as_ref())) }
        {
            f.debug_struct(name)
                .field("ty", &creator.create_owned_type(self.tid))
                .finish()
        } else {
            f.debug_struct(name).field("tid", &self.tid).finish()
        }
    }

    fn name(&self) -> &'static str {
        "Enum"
    }
}

impl AttributeHelper for Enum {
    fn get_attributes(&self) -> Option<&Vec<syn::Attribute>> {
        Some(&self.as_syn().attrs)
    }

    fn get_mut_attributes(&mut self) -> Option<&mut Vec<syn::Attribute>> {
        None // Read only
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RawEnum {
    /// Read-only pointer to a syntax tree node that never changes.
    pub(crate) ptr_enum: NonNull<syn::ItemEnum>,

    /// Refer to the [module documentation](self).
    pub(crate) vis_node: Option<NodeIndex>,
}

impl RawEnum {
    pub(crate) fn as_syn<'o>(&self) -> &'o syn::ItemEnum {
        unsafe { self.ptr_enum.as_ref() }
    }

    pub(crate) fn syn_id(&self) -> SynId {
        self.as_syn().syn_id()
    }

    pub(crate) fn visibility(&self) -> PathVis {
        PathVis::new(&self.as_syn().vis)
    }
}

impl fmt::Debug for RawEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name)
            .field("vis_node", &self.vis_node)
            .finish()
    }
}

impl format::DebugBriefly for RawEnum {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name).finish()
    }

    fn name(&self) -> &'static str {
        "RawEnum"
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Field {
    /// Read-only pointer to a syntax tree node that never changes.
    pub(crate) ptr_field: NonNull<syn::Field>,

    /// Refer to the [module documentation](self).
    pub(crate) vis_node: NodeIndex,

    /// Type of this item.
    pub(crate) tid: TypeId,
}

impl Field {
    pub fn ptr_syn(&self) -> *const syn::Field {
        self.ptr_field.as_ptr().cast_const()
    }

    pub fn as_syn<'o>(&self) -> &'o syn::Field {
        unsafe { self.ptr_field.as_ref() }
    }

    pub fn syn_id(&self) -> SynId {
        self.as_syn().syn_id()
    }
}

impl fmt::Debug for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        if let Some(creator) =
            unsafe { OWNED_TYPE_CREATOR.with(|creator| creator.get().map(|ptr| ptr.as_ref())) }
        {
            f.debug_struct(name)
                .field("vis_node", &self.vis_node)
                .field("ty", &creator.create_owned_type(self.tid))
                .finish()
        } else {
            f.debug_struct(name)
                .field("vis_node", &self.vis_node)
                .field("tid", &self.tid)
                .finish()
        }
    }
}

impl format::DebugBriefly for Field {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        if let Some(creator) =
            unsafe { OWNED_TYPE_CREATOR.with(|creator| creator.get().map(|ptr| ptr.as_ref())) }
        {
            f.debug_struct(name)
                .field("ty", &creator.create_owned_type(self.tid))
                .finish()
        } else {
            f.debug_struct(name).field("tid", &self.tid).finish()
        }
    }

    fn name(&self) -> &'static str {
        "Field"
    }
}

impl AttributeHelper for Field {
    fn get_attributes(&self) -> Option<&Vec<syn::Attribute>> {
        Some(&self.as_syn().attrs)
    }

    fn get_mut_attributes(&mut self) -> Option<&mut Vec<syn::Attribute>> {
        None // Read only
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RawField {
    /// Read-only pointer to a syntax tree node that never changes.
    pub(crate) ptr_field: NonNull<syn::Field>,

    /// Refer to the [module documentation](self).
    pub(crate) vis_node: Option<NodeIndex>,
}

impl RawField {
    pub(crate) fn as_syn<'o>(&self) -> &'o syn::Field {
        unsafe { self.ptr_field.as_ref() }
    }

    pub(crate) fn syn_id(&self) -> SynId {
        self.as_syn().syn_id()
    }

    pub(crate) fn visibility(&self) -> PathVis {
        PathVis::new(&self.as_syn().vis)
    }
}

impl fmt::Debug for RawField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name)
            .field("vis_node", &self.vis_node)
            .finish()
    }
}

impl format::DebugBriefly for RawField {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name).finish()
    }

    fn name(&self) -> &'static str {
        "RawField"
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Fn {
    /// Read-only pointer to a syntax tree node that never changes.
    pub(crate) ptr_attr: NonNull<Vec<syn::Attribute>>,
    pub(crate) ptr_sig: NonNull<syn::Signature>,
    pub(crate) ptr_block: NonNull<syn::Block>,

    /// Refer to the [module documentation](self).
    pub(crate) vis_node: NodeIndex,

    /// Type of this item.
    pub(crate) tid: TypeId,
}

impl Fn {
    pub fn ptr_syn_attr(&self) -> *const Vec<syn::Attribute> {
        self.ptr_attr.as_ptr().cast_const()
    }

    pub fn ptr_syn_sig(&self) -> *const syn::Signature {
        self.ptr_sig.as_ptr().cast_const()
    }

    pub fn ptr_syn_block(&self) -> *const syn::Block {
        self.ptr_block.as_ptr().cast_const()
    }

    pub fn syn_attr<'o>(&self) -> &'o Vec<syn::Attribute> {
        unsafe { self.ptr_attr.as_ref() }
    }

    pub fn syn_sig<'o>(&self) -> &'o syn::Signature {
        unsafe { self.ptr_sig.as_ref() }
    }

    pub fn syn_block<'o>(&self) -> &'o syn::Block {
        unsafe { self.ptr_block.as_ref() }
    }

    pub fn syn_id(&self) -> SynId {
        self.syn_block().syn_id()
    }

    pub fn type_id(&self) -> TypeId {
        self.tid
    }
}

impl fmt::Debug for Fn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        if let Some(creator) =
            unsafe { OWNED_TYPE_CREATOR.with(|creator| creator.get().map(|ptr| ptr.as_ref())) }
        {
            f.debug_struct(name)
                .field("vis_node", &self.vis_node)
                .field("ty", &creator.create_owned_type(self.tid))
                .finish()
        } else {
            f.debug_struct(name)
                .field("vis_node", &self.vis_node)
                .field("tid", &self.tid)
                .finish()
        }
    }
}

impl format::DebugBriefly for Fn {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        if let Some(creator) =
            unsafe { OWNED_TYPE_CREATOR.with(|creator| creator.get().map(|ptr| ptr.as_ref())) }
        {
            f.debug_struct(name)
                .field("ty", &creator.create_owned_type(self.tid))
                .finish()
        } else {
            f.debug_struct(name).field("tid", &self.tid).finish()
        }
    }

    fn name(&self) -> &'static str {
        "Fn"
    }
}

impl AttributeHelper for Fn {
    fn get_attributes(&self) -> Option<&Vec<syn::Attribute>> {
        Some(self.syn_attr())
    }

    fn get_mut_attributes(&mut self) -> Option<&mut Vec<syn::Attribute>> {
        None // Read only
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RawFn {
    /// Read-only pointer to a syntax tree node that never changes.
    pub(crate) ptr_attr: NonNull<Vec<syn::Attribute>>,
    pub(crate) ptr_vis: NonNull<syn::Visibility>,
    pub(crate) ptr_sig: NonNull<syn::Signature>,
    pub(crate) ptr_block: NonNull<syn::Block>,

    /// Refer to the [module documentation](self).
    pub(crate) vis_node: Option<NodeIndex>,

    /// TODO: Need comment about this at the module level.
    //
    // Node index to the parent module.
    pub(crate) unscoped_base: Option<NodeIndex>,
}

impl RawFn {
    pub(crate) fn as_syn_vis<'o>(&self) -> &'o syn::Visibility {
        unsafe { self.ptr_vis.as_ref() }
    }

    pub(crate) fn as_syn_sig<'o>(&self) -> &'o syn::Signature {
        unsafe { self.ptr_sig.as_ref() }
    }

    pub(crate) fn as_syn_block<'o>(&self) -> &'o syn::Block {
        unsafe { self.ptr_block.as_ref() }
    }

    pub(crate) fn syn_id(&self) -> SynId {
        self.as_syn_block().syn_id()
    }

    pub(crate) fn visibility(&self) -> PathVis {
        PathVis::new(self.as_syn_vis())
    }
}

impl fmt::Debug for RawFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name)
            .field("vis_node", &self.vis_node)
            .field("unscoped_base", &self.unscoped_base)
            .finish()
    }
}

impl format::DebugBriefly for RawFn {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name).finish()
    }

    fn name(&self) -> &'static str {
        "RawFn"
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Local {
    /// Read-only pointer to a syntax tree node that never changes.
    pub(crate) ptr_attr: NonNull<Vec<syn::Attribute>>,
    pub(crate) ptr_ident: Which2<NonNull<syn::PatIdent>, NonNull<syn::Receiver>>,

    /// Optional explicit syn type that may attach to the local.
    pub(crate) ptr_ty: Option<NonNull<syn::Type>>,

    /// Type of this item.
    pub(crate) tid: TypeId,
}

impl Local {
    pub fn ptr_syn_attr(&self) -> *const Vec<syn::Attribute> {
        self.ptr_attr.as_ptr().cast_const()
    }

    pub fn ptr_syn_ident(&self) -> Which2<*const syn::PatIdent, *const syn::Receiver> {
        match self.ptr_ident {
            Which2::A(ptr) => Which2::A(ptr.as_ptr().cast_const()),
            Which2::B(ptr) => Which2::B(ptr.as_ptr().cast_const()),
        }
    }

    pub fn ptr_syn_ty(&self) -> Option<*const syn::Type> {
        self.ptr_ty.map(|ptr| ptr.as_ptr().cast_const())
    }

    pub fn as_syn<'o>(&self) -> Which2<&'o syn::PatIdent, &'o syn::Receiver> {
        match &self.ptr_ident {
            Which2::A(ptr) => Which2::A(unsafe { ptr.as_ref() }),
            Which2::B(ptr) => Which2::B(unsafe { ptr.as_ref() }),
        }
    }

    pub fn as_syn_attr<'o>(&self) -> &'o Vec<syn::Attribute> {
        unsafe { self.ptr_attr.as_ref() }
    }

    pub fn syn_id(&self) -> SynId {
        match self.as_syn() {
            Which2::A(pat_ident) => pat_ident.syn_id(),
            Which2::B(recv) => recv.syn_id(),
        }
    }

    pub fn syn_type<'o>(&self) -> Option<&'o syn::Type> {
        self.ptr_ty.map(|ptr| unsafe { ptr.as_ref() })
    }
}

impl fmt::Debug for Local {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        if let Some(creator) =
            unsafe { OWNED_TYPE_CREATOR.with(|creator| creator.get().map(|ptr| ptr.as_ref())) }
        {
            f.debug_struct(name)
                .field("ty", &creator.create_owned_type(self.tid))
                .finish()
        } else {
            f.debug_struct(name).field("tid", &self.tid).finish()
        }
    }
}

impl format::DebugBriefly for Local {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        if let Some(creator) =
            unsafe { OWNED_TYPE_CREATOR.with(|creator| creator.get().map(|ptr| ptr.as_ref())) }
        {
            f.debug_struct(name)
                .field("ty", &creator.create_owned_type(self.tid))
                .finish()
        } else {
            f.debug_struct(name).field("tid", &self.tid).finish()
        }
    }

    fn name(&self) -> &'static str {
        "Local"
    }
}

impl AttributeHelper for Local {
    fn get_attributes(&self) -> Option<&Vec<syn::Attribute>> {
        Some(self.as_syn_attr())
    }

    fn get_mut_attributes(&mut self) -> Option<&mut Vec<syn::Attribute>> {
        None // Read only
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RawLocal {
    /// Read-only pointer to a syntax tree node that never changes.
    pub(crate) ptr_attr: NonNull<Vec<syn::Attribute>>,
    pub(crate) ptr_ident: Which2<NonNull<syn::PatIdent>, NonNull<syn::Receiver>>,

    /// Optional explicit syn type that may attach to the local.
    pub(crate) ptr_ty: Option<NonNull<syn::Type>>,

    pub(crate) mut_: bool,
}

impl RawLocal {
    pub(crate) fn as_syn<'o>(&self) -> Which2<&'o syn::PatIdent, &'o syn::Receiver> {
        match &self.ptr_ident {
            Which2::A(ptr) => Which2::A(unsafe { ptr.as_ref() }),
            Which2::B(ptr) => Which2::B(unsafe { ptr.as_ref() }),
        }
    }

    pub(crate) fn syn_id(&self) -> SynId {
        match self.as_syn() {
            Which2::A(pat_ident) => pat_ident.syn_id(),
            Which2::B(recv) => recv.syn_id(),
        }
    }
}

impl fmt::Debug for RawLocal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name).field("mut", &self.mut_).finish()
    }
}

impl format::DebugBriefly for RawLocal {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name).finish()
    }

    fn name(&self) -> &'static str {
        "RawLocal"
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Mod {
    /// Read-only pointer to a syntax tree node that never changes.
    ///
    /// This points to 'mod', not a file, so if the module was loaded as a file, then this could
    /// be None.
    pub(crate) ptr_mod: Option<NonNull<syn::ItemMod>>,

    /// Pointer to a file if the module was loaded as a file, not by 'mod'.
    pub(crate) ptr_file: Option<NonNull<syn::File>>,

    /// Refer to the [module documentation](self).
    pub(crate) vis_node: NodeIndex,

    /// Absolute file path that represent this module.
    ///
    /// ```ignore
    /// // /a.rs
    /// mod b {           // fpath: /a/b
    ///     mod c;        // fpath: /a/b/c.rs or /a/b/c/mod.rs
    /// }
    /// #[path = "d.rs"]
    /// mod d;            // fpath: /d.rs
    /// mod e;            // fpath: /a/e.rs or /a/e/mod.rs
    /// ```
    pub(crate) fpath: PathBuf,

    /// - True if the file is one of "mod.rs", "main.rs", or "lib.rs".
    /// - True if the file is determined by "path" attribute and the module is not inline (e.g.
    ///   #[path = "a.rs"] mod foo;)
    /// - False otherwise.
    pub(crate) mod_rs: bool,
}

impl Mod {
    pub fn as_syn_mod<'o>(&self) -> Option<&'o syn::ItemMod> {
        self.ptr_mod.map(|ptr| unsafe { ptr.as_ref() })
    }

    pub fn as_syn_file<'o>(&self) -> Option<&'o syn::File> {
        self.ptr_file.map(|ptr| unsafe { ptr.as_ref() })
    }

    pub fn syn_id(&self) -> SynId {
        // Real mod must have one of the two pointers, but virtual mod, like root, may not.
        self.ptr_mod
            .map(|ptr| unsafe { ptr.as_ref().syn_id() })
            .unwrap_or(unsafe { self.ptr_file.expect("nullptr Mod").as_ref().syn_id() })
    }

    pub fn file_path(&self) -> &StdPath {
        self.fpath.as_path()
    }

    pub fn is_inline(&self) -> bool {
        if let Some(item_mod) = self.as_syn_mod() {
            item_mod.content.is_some()
        } else {
            false
        }
    }
}

impl fmt::Debug for Mod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name)
            .field("vis_node", &self.vis_node)
            .field("fpath", &self.fpath)
            .field("mod_rs", &self.mod_rs)
            .finish()
    }
}

impl format::DebugBriefly for Mod {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name).finish()
    }

    fn name(&self) -> &'static str {
        "Mod"
    }
}

impl AttributeHelper for Mod {
    fn get_attributes(&self) -> Option<&Vec<syn::Attribute>> {
        self.as_syn_mod().map(|item_mod| &item_mod.attrs)
    }

    fn get_mut_attributes(&mut self) -> Option<&mut Vec<syn::Attribute>> {
        None // Read only
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RawMod {
    /// Read-only pointer to a syntax tree node that never changes.
    ///
    /// This points to 'mod', not a file, so if the module was loaded as a file, then this could
    /// be None.
    pub(crate) ptr_mod: Option<NonNull<syn::ItemMod>>,

    /// Pointer to a file if the module was loaded as a file, not by 'mod'.
    pub(crate) ptr_file: Option<NonNull<syn::File>>,

    /// Refer to the [module documentation](self).
    pub(crate) vis_node: Option<NodeIndex>,
    pub(crate) fpath: PathBuf,
    pub(crate) mod_rs: bool,
}

impl RawMod {
    pub(crate) fn as_syn_mod<'o>(&self) -> Option<&'o syn::ItemMod> {
        self.ptr_mod.map(|ptr| unsafe { ptr.as_ref() })
    }

    pub(crate) fn syn_id(&self) -> SynId {
        // Real mod must have one of the two pointers, but virtual mod, like root, may not.
        self.ptr_mod
            .map(|ptr| unsafe { ptr.as_ref().syn_id() })
            .unwrap_or(unsafe { self.ptr_file.expect("nullptr Mod").as_ref().syn_id() })
    }

    pub(crate) fn is_inline(&self) -> bool {
        if let Some(item_mod) = self.as_syn_mod() {
            item_mod.content.is_some()
        } else {
            false
        }
    }

    pub(crate) fn visibility(&self) -> PathVis {
        if let Some(item_mod) = self.as_syn_mod() {
            PathVis::new(&item_mod.vis)
        } else {
            PathVis::Private
        }
    }
}

impl fmt::Debug for RawMod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name)
            .field("vis_node", &self.vis_node)
            .field("fpath", &self.fpath)
            .field("mod_rs", &self.mod_rs)
            .finish()
    }
}

impl format::DebugBriefly for RawMod {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name).finish()
    }

    fn name(&self) -> &'static str {
        "RawMod"
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Struct {
    /// Read-only pointer to a syntax tree node that never changes.
    pub(crate) ptr_struct: NonNull<syn::ItemStruct>,

    /// Refer to the [module documentation](self).
    pub(crate) vis_node: NodeIndex,

    /// Type of this item.
    pub(crate) tid: TypeId,
}

impl Struct {
    pub fn ptr_syn(&self) -> *const syn::ItemStruct {
        self.ptr_struct.as_ptr().cast_const()
    }

    pub fn as_syn<'o>(&self) -> &'o syn::ItemStruct {
        unsafe { self.ptr_struct.as_ref() }
    }

    pub fn syn_id(&self) -> SynId {
        self.as_syn().syn_id()
    }

    pub fn type_id(&self) -> TypeId {
        self.tid
    }
}

impl fmt::Debug for Struct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        if let Some(creator) =
            unsafe { OWNED_TYPE_CREATOR.with(|creator| creator.get().map(|ptr| ptr.as_ref())) }
        {
            f.debug_struct(name)
                .field("vis_node", &self.vis_node)
                .field("ty", &creator.create_owned_type(self.tid))
                .finish()
        } else {
            f.debug_struct(name)
                .field("vis_node", &self.vis_node)
                .field("tid", &self.tid)
                .finish()
        }
    }
}

impl format::DebugBriefly for Struct {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name).finish()
    }

    fn name(&self) -> &'static str {
        "Struct"
    }
}

impl AttributeHelper for Struct {
    fn get_attributes(&self) -> Option<&Vec<syn::Attribute>> {
        Some(&self.as_syn().attrs)
    }

    fn get_mut_attributes(&mut self) -> Option<&mut Vec<syn::Attribute>> {
        None // Read only
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RawStruct {
    /// Read-only pointer to a syntax tree node that never changes.
    pub(crate) ptr_struct: NonNull<syn::ItemStruct>,

    /// Refer to the [module documentation](self).
    pub(crate) vis_node: Option<NodeIndex>,
}

impl RawStruct {
    pub(crate) fn as_syn<'o>(&self) -> &'o syn::ItemStruct {
        unsafe { self.ptr_struct.as_ref() }
    }

    pub(crate) fn syn_id(&self) -> SynId {
        self.as_syn().syn_id()
    }

    pub(crate) fn visibility(&self) -> PathVis {
        PathVis::new(&self.as_syn().vis)
    }
}

impl fmt::Debug for RawStruct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name)
            .field("vis_node", &self.vis_node)
            .finish()
    }
}

impl format::DebugBriefly for RawStruct {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name).finish()
    }

    fn name(&self) -> &'static str {
        "RawStruct"
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Trait {
    /// Read-only pointer to a syntax tree node that never changes.
    pub(crate) ptr_trait: NonNull<syn::ItemTrait>,

    /// Refer to the [module documentation](self).
    pub(crate) vis_node: NodeIndex,
}

impl Trait {
    pub fn ptr_syn(&self) -> *const syn::ItemTrait {
        self.ptr_trait.as_ptr().cast_const()
    }

    pub fn as_syn<'o>(&self) -> &'o syn::ItemTrait {
        unsafe { self.ptr_trait.as_ref() }
    }

    pub fn syn_id(&self) -> SynId {
        self.as_syn().syn_id()
    }
}

impl fmt::Debug for Trait {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name)
            .field("vis_node", &self.vis_node)
            .finish()
    }
}

impl format::DebugBriefly for Trait {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name).finish()
    }

    fn name(&self) -> &'static str {
        "Trait"
    }
}

impl AttributeHelper for Trait {
    fn get_attributes(&self) -> Option<&Vec<syn::Attribute>> {
        Some(&self.as_syn().attrs)
    }

    fn get_mut_attributes(&mut self) -> Option<&mut Vec<syn::Attribute>> {
        None // Read only
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RawTrait {
    /// Read-only pointer to a syntax tree node that never changes.
    pub(crate) ptr_trait: NonNull<syn::ItemTrait>,

    /// Refer to the [module documentation](self).
    pub(crate) vis_node: Option<NodeIndex>,
}

impl RawTrait {
    pub(crate) fn as_syn<'o>(&self) -> &'o syn::ItemTrait {
        unsafe { self.ptr_trait.as_ref() }
    }

    pub(crate) fn syn_id(&self) -> SynId {
        self.as_syn().syn_id()
    }

    pub(crate) fn visibility(&self) -> PathVis {
        PathVis::new(&self.as_syn().vis)
    }
}

impl fmt::Debug for RawTrait {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name)
            .field("vis_node", &self.vis_node)
            .finish()
    }
}

impl format::DebugBriefly for RawTrait {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name).finish()
    }

    fn name(&self) -> &'static str {
        "RawTrait"
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct TypeAlias {
    /// Read-only pointer to a syntax tree node that never changes.
    pub(crate) ptr_type: NonNull<syn::ItemType>,

    /// Refer to the [module documentation](self).
    pub(crate) vis_node: NodeIndex,

    /// Type of this item.
    pub(crate) tid: TypeId,
}

impl TypeAlias {
    pub fn ptr_syn(&self) -> *const syn::ItemType {
        self.ptr_type.as_ptr().cast_const()
    }

    pub fn as_syn<'o>(&self) -> &'o syn::ItemType {
        unsafe { self.ptr_type.as_ref() }
    }

    pub fn syn_id(&self) -> SynId {
        self.as_syn().syn_id()
    }
}

impl fmt::Debug for TypeAlias {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        if let Some(creator) =
            unsafe { OWNED_TYPE_CREATOR.with(|creator| creator.get().map(|ptr| ptr.as_ref())) }
        {
            f.debug_struct(name)
                .field("vis_node", &self.vis_node)
                .field("ty", &creator.create_owned_type(self.tid))
                .finish()
        } else {
            f.debug_struct(name)
                .field("vis_node", &self.vis_node)
                .field("tid", &self.tid)
                .finish()
        }
    }
}

impl format::DebugBriefly for TypeAlias {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        if let Some(creator) =
            unsafe { OWNED_TYPE_CREATOR.with(|creator| creator.get().map(|ptr| ptr.as_ref())) }
        {
            f.debug_struct(name)
                .field("ty", &creator.create_owned_type(self.tid))
                .finish()
        } else {
            f.debug_struct(name).field("tid", &self.tid).finish()
        }
    }

    fn name(&self) -> &'static str {
        "TypeAlias"
    }
}

impl AttributeHelper for TypeAlias {
    fn get_attributes(&self) -> Option<&Vec<syn::Attribute>> {
        Some(&self.as_syn().attrs)
    }

    fn get_mut_attributes(&mut self) -> Option<&mut Vec<syn::Attribute>> {
        None // Read only
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RawTypeAlias {
    /// Read-only pointer to a syntax tree node that never changes.
    pub(crate) ptr_type: NonNull<syn::ItemType>,

    /// Refer to the [module documentation](self).
    pub(crate) vis_node: Option<NodeIndex>,
}

impl RawTypeAlias {
    pub(crate) fn as_syn<'o>(&self) -> &'o syn::ItemType {
        unsafe { self.ptr_type.as_ref() }
    }

    pub(crate) fn syn_id(&self) -> SynId {
        self.as_syn().syn_id()
    }

    pub(crate) fn visibility(&self) -> PathVis {
        PathVis::new(&self.as_syn().vis)
    }
}

impl fmt::Debug for RawTypeAlias {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name)
            .field("vis_node", &self.vis_node)
            .finish()
    }
}

impl format::DebugBriefly for RawTypeAlias {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name).finish()
    }

    fn name(&self) -> &'static str {
        "RawTypeAlias"
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Use {
    /// Read-only pointer to a syntax tree node that never changes.
    pub(crate) ptr_group: NonNull<syn::ItemUse>,

    /// Abstract pointer to a specific item in the [`syn::ItemUse`].
    ///
    /// For instance, if [`Self::syn_group`] points to 'use a::{b, c, \*}', this field points to
    /// 'b', 'c', or '\*'.
    pub(crate) syn_part: SynId,

    /// Refer to the [module documentation](self).
    pub(crate) vis_node: NodeIndex,
    pub(crate) dst: PathId,
}

impl Use {
    pub fn ptr_syn(&self) -> *const syn::ItemUse {
        self.ptr_group.as_ptr().cast_const()
    }

    pub fn syn_id(&self) -> SynId {
        self.syn_part
    }
}

impl fmt::Debug for Use {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name)
            .field("vis_node", &self.vis_node)
            .field("dst", &self.dst)
            .finish()
    }
}

impl format::DebugBriefly for Use {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name).field("dst", &self.dst).finish()
    }

    fn name(&self) -> &'static str {
        "Use"
    }
}

impl AttributeHelper for Use {
    fn get_attributes(&self) -> Option<&Vec<syn::Attribute>> {
        unsafe { Some(&self.ptr_group.as_ref().attrs) }
    }

    fn get_mut_attributes(&mut self) -> Option<&mut Vec<syn::Attribute>> {
        None // Read only
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RawUse {
    /// Read-only pointer to a syntax tree node that never changes.
    pub(crate) ptr_group: NonNull<syn::ItemUse>,

    /// Abstract pointer to a specific item in the [`syn::ItemUse`].
    ///
    /// For instance, if [`Self::syn_group`] points to 'use a::{b, c, \*}', this field points to
    /// 'b', 'c', or '\*'.
    pub(crate) syn_part: SynId,

    /// Refer to the [module documentation](self).
    pub(crate) vis_node: Option<NodeIndex>,
    pub(crate) npath: String,
    pub(crate) dst_node: Option<NodeIndex>,
}

impl RawUse {
    pub(crate) fn syn_id(&self) -> SynId {
        self.syn_part
    }

    pub(crate) fn visibility(&self) -> PathVis {
        let group = unsafe { self.ptr_group.as_ref() };
        PathVis::new(&group.vis)
    }
}

impl RawUse {
    pub(crate) fn is_glob(&self) -> bool {
        self.npath.ends_with("*")
    }
}

impl fmt::Debug for RawUse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name)
            .field("vis_node", &self.vis_node)
            .field("npath", &self.npath)
            .field("dst_node", &self.dst_node)
            .finish()
    }
}

impl format::DebugBriefly for RawUse {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name).finish()
    }

    fn name(&self) -> &'static str {
        "RawUse"
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Variant {
    /// Read-only pointer to a syntax tree node that never changes.
    pub(crate) ptr_variant: NonNull<syn::Variant>,

    /// Refer to the [module documentation](self).
    pub(crate) vis_node: NodeIndex,

    /// Type of this item. This is the same id as enum's for now.
    pub(crate) tid: TypeId,

    pub(crate) nth: usize,

    /// Discriminant of the variant.
    pub(crate) disc: isize,
}

impl Variant {
    pub fn ptr_syn(&self) -> *const syn::Variant {
        self.ptr_variant.as_ptr().cast_const()
    }

    pub fn as_syn<'o>(&self) -> &'o syn::Variant {
        unsafe { self.ptr_variant.as_ref() }
    }

    pub fn syn_id(&self) -> SynId {
        self.as_syn().syn_id()
    }

    pub fn type_id(&self) -> TypeId {
        self.tid
    }
}

impl fmt::Debug for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        if let Some(creator) =
            unsafe { OWNED_TYPE_CREATOR.with(|creator| creator.get().map(|ptr| ptr.as_ref())) }
        {
            f.debug_struct(name)
                .field("vis_node", &self.vis_node)
                .field("ty", &creator.create_owned_type(self.tid))
                .field("disc", &self.disc)
                .finish()
        } else {
            f.debug_struct(name)
                .field("vis_node", &self.vis_node)
                .field("tid", &self.tid)
                .field("disc", &self.disc)
                .finish()
        }
    }
}

impl format::DebugBriefly for Variant {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        if let Some(creator) =
            unsafe { OWNED_TYPE_CREATOR.with(|creator| creator.get().map(|ptr| ptr.as_ref())) }
        {
            f.debug_struct(name)
                .field("ty", &creator.create_owned_type(self.tid))
                .finish()
        } else {
            f.debug_struct(name).field("tid", &self.tid).finish()
        }
    }

    fn name(&self) -> &'static str {
        "Variant"
    }
}

impl AttributeHelper for Variant {
    fn get_attributes(&self) -> Option<&Vec<syn::Attribute>> {
        Some(&self.as_syn().attrs)
    }

    fn get_mut_attributes(&mut self) -> Option<&mut Vec<syn::Attribute>> {
        None // Read only
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RawVariant {
    /// Read-only pointer to a syntax tree node that never changes.
    pub(crate) ptr_variant: NonNull<syn::Variant>,

    /// Refer to the [module documentation](self).
    pub(crate) vis_node: Option<NodeIndex>,

    pub(crate) nth: usize,
}

impl RawVariant {
    pub(crate) fn as_syn<'o>(&self) -> &'o syn::Variant {
        unsafe { self.ptr_variant.as_ref() }
    }

    pub(crate) fn syn_id(&self) -> SynId {
        self.as_syn().syn_id()
    }
}

impl fmt::Debug for RawVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name)
            .field("vis_node", &self.vis_node)
            .finish()
    }
}

impl format::DebugBriefly for RawVariant {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, _filter: &PrintFilter) -> fmt::Result {
        let name = format::DebugBriefly::name(self);
        f.debug_struct(name).finish()
    }

    fn name(&self) -> &'static str {
        "RawVariant"
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemIndex(pub(crate) usize);

impl From<usize> for ItemIndex {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl fmt::Display for ItemIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for ItemIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq<usize> for ItemIndex {
    fn eq(&self, other: &usize) -> bool {
        self.0.eq(other)
    }
}

impl PartialOrd<usize> for ItemIndex {
    fn partial_cmp(&self, other: &usize) -> Option<cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl ops::Add<usize> for ItemIndex {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl ops::AddAssign<usize> for ItemIndex {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl ops::Index<ItemIndex> for [PrivItem] {
    type Output = PrivItem;

    fn index(&self, index: ItemIndex) -> &Self::Output {
        &self[index.0]
    }
}

impl ops::IndexMut<ItemIndex> for [PrivItem] {
    fn index_mut(&mut self, index: ItemIndex) -> &mut Self::Output {
        &mut self[index.0]
    }
}

impl ops::Index<ItemIndex> for SmallVec<[PrivItem; 1]> {
    type Output = PrivItem;

    fn index(&self, index: ItemIndex) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl ops::IndexMut<ItemIndex> for SmallVec<[PrivItem; 1]> {
    fn index_mut(&mut self, index: ItemIndex) -> &mut Self::Output {
        &mut self.as_mut_slice()[index]
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) enum PathVis {
    Pub,
    PubCrate,
    PubSuper,
    PubPath(String),
    #[default]
    Private,
}

impl PathVis {
    pub(crate) fn new(vis: &syn::Visibility) -> Self {
        match vis {
            syn::Visibility::Public(_) => PathVis::Pub,
            syn::Visibility::Restricted(syn::VisRestricted { path, .. }) => {
                let path = util::get_name_path_from_syn_path(path);
                match path.as_str() {
                    "crate" => PathVis::PubCrate,
                    "super" => PathVis::PubSuper,
                    "self" => PathVis::Private,
                    _ => PathVis::PubPath(path),
                }
            }
            syn::Visibility::Inherited => PathVis::Private,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConstPtr {
    Free(*const syn::ItemConst),
    Inher(*const syn::ImplItemConst),
    TraitDefault(*const syn::TraitItemConst),
    TraitImpl(*const syn::ImplItemConst),
}

pub trait ItemTrait {
    /// Returns true if both items are made from the same syn node and resolved data will be the
    /// same.
    ///
    /// For example, [`PrivItem::Fn`] and [`PrivItem::RawFn`] can be the same if they are made from
    /// the same syn function node.
    fn is_effective_same(&self, other: &Self) -> bool;

    /// Returns a kind of the item regardless of whether the item is resolved or not.
    ///
    /// For example, [`PrivItem::Fn`] and [`PrivItem::RawFn`] returns the same
    /// [`EffectiveItemKind::Fn`].
    fn effective_kind(&self) -> EffectiveItemKind;

    fn vis_node(&self) -> TriOption<NodeIndex, ()>;
    fn type_id(&self) -> TriOption<TypeId, ()>;
    fn syn_id(&self) -> Option<SynId>;

    fn as_fn(&self) -> Option<&Fn>;
    fn as_struct(&self) -> Option<&Struct>;
    fn as_type_alias(&self) -> Option<&TypeAlias>;
    fn as_use(&self) -> Option<&Use>;
    fn as_variant(&self) -> Option<&Variant>;
    fn as_mut_mod(&mut self) -> Option<&mut Mod>;
}

impl ItemTrait for PrivItem {
    #[rustfmt::skip]
    fn is_effective_same(&self, other: &Self) -> bool {
        let (l, r) = (self, other);
        match l {
            Self::Block(l) => matches!(r, Self::Block(r) if l.ptr_block == r.ptr_block),
            Self::Const(l) => match r {
                Self::Const(r) => l.ptr_syn() == r.ptr_syn(),
                Self::RawConst(r) => l.ptr_syn() == r.ptr_syn(),
                _ => false,
            },
            Self::Enum(Enum { ptr_enum: l_ptr, .. }) => matches!(r,
                Self::Enum(Enum { ptr_enum: r_ptr, .. })
                | Self::RawEnum(RawEnum { ptr_enum: r_ptr, .. }) if l_ptr == r_ptr
            ),
            Self::Field(Field { ptr_field: l_ptr, .. }) => matches!(r,
                Self::Field(Field { ptr_field: r_ptr, .. })
                | Self::RawField(RawField { ptr_field: r_ptr, .. }) if l_ptr == r_ptr,
            ),
            Self::Fn(l) => match r {
                Self::Fn(r) => l.syn_id() == r.syn_id(),
                Self::RawFn(r) => l.syn_id() == r.syn_id(),
                _ => false,
            },
            Self::Local(Local { ptr_ident: l_ptr, .. }) => matches!(r,
                Self::Local(Local { ptr_ident: r_ptr, .. })
                | Self::RawLocal(RawLocal { ptr_ident: r_ptr, .. }) if l_ptr == r_ptr,
            ),
            Self::Mod(l) => match r {
                Self::Mod(r) => l.syn_id() == r.syn_id(),
                Self::RawMod(r) => l.syn_id() == r.syn_id(),
                _ => false,
            },
            Self::Struct(Struct { ptr_struct: l_ptr, .. }) => matches!(r,
                Self::Struct(Struct { ptr_struct: r_ptr, .. })
                | Self::RawStruct(RawStruct { ptr_struct: r_ptr, .. }) if l_ptr == r_ptr,
            ),
            Self::Trait(Trait { ptr_trait: l_ptr, .. }) => matches!(r,
                Self::Trait(Trait { ptr_trait: r_ptr, .. })
                | Self::RawTrait(RawTrait { ptr_trait: r_ptr, .. }) if l_ptr == r_ptr,
            ),
            Self::TypeAlias(TypeAlias { ptr_type: l_ptr, .. }) => matches!(r,
                Self::TypeAlias(TypeAlias { ptr_type: r_ptr, .. })
                | Self::RawTypeAlias(RawTypeAlias { ptr_type: r_ptr, .. }) if l_ptr == r_ptr,
            ),
            Self::Use(l) => match r {
                Self::Use(r) => l.syn_id() == r.syn_id() && l.dst == r.dst,
                Self::RawUse(r) => !r.is_glob() && l.syn_id() == r.syn_id(),
                _ => false,
            },
            Self::Variant(Variant { ptr_variant: l_ptr, .. }) => matches!(r,
                Self::Variant(Variant { ptr_variant: r_ptr, .. })
                | Self::RawVariant(RawVariant { ptr_variant: r_ptr, .. }) if l_ptr == r_ptr,
            ),
            Self::RawConst(l) => match r {
                Self::Const(r) => l.ptr_syn() == r.ptr_syn(),
                Self::RawConst(r) => l.ptr_syn() == r.ptr_syn(),
                _ => false,
            },
            Self::RawEnum(RawEnum { ptr_enum: l_ptr, .. }) => matches!(r,
                Self::Enum(Enum { ptr_enum: r_ptr, .. })
                | Self::RawEnum(RawEnum { ptr_enum: r_ptr, .. }) if l_ptr == r_ptr,
            ),
            Self::RawField(RawField { ptr_field: l_ptr, .. }) => matches!(r,
                Self::Field(Field { ptr_field: r_ptr, .. })
                | Self::RawField(RawField { ptr_field: r_ptr, .. }) if l_ptr == r_ptr,
            ),
            Self::RawFn(l) => match r {
                Self::Fn(r) => l.syn_id() == r.syn_id(),
                Self::RawFn(r) => l.syn_id() == r.syn_id(),
                _ => false,
            },
            Self::RawLocal(RawLocal { ptr_ident: l_ptr, .. }) => matches!(r,
                Self::Local(Local { ptr_ident: r_ptr, .. })
                | Self::RawLocal(RawLocal { ptr_ident: r_ptr, .. }) if l_ptr == r_ptr,
            ),
            Self::RawMod(l) => match r {
                Self::Mod(r) => l.syn_id() == r.syn_id(),
                Self::RawMod(r) => l.syn_id() == r.syn_id(),
                _ => false,
            },
            Self::RawStruct(RawStruct { ptr_struct: l_ptr, .. }) => matches!(r,
                Self::Struct(Struct { ptr_struct: r_ptr, .. })
                | Self::RawStruct(RawStruct { ptr_struct: r_ptr, .. }) if l_ptr == r_ptr,
            ),
            Self::RawTrait(RawTrait { ptr_trait: l_ptr, .. }) => matches!(r,
                Self::Trait(Trait { ptr_trait: r_ptr, .. })
                | Self::RawTrait(RawTrait { ptr_trait: r_ptr, .. }) if l_ptr == r_ptr,
            ),
            Self::RawTypeAlias(RawTypeAlias { ptr_type: l_ptr, .. }) => matches!(r,
                Self::TypeAlias(TypeAlias { ptr_type: r_ptr, .. })
                | Self::RawTypeAlias(RawTypeAlias { ptr_type: r_ptr, .. }) if l_ptr == r_ptr,
            ),
            Self::RawUse(l) => match r {
                Self::Use(r) => !l.is_glob() && l.syn_id() == r.syn_id(),
                Self::RawUse(r) => l.syn_id() == r.syn_id(),
                _ => false,
            },
            Self::RawVariant(RawVariant { ptr_variant: l_ptr, .. }) => matches!(r,
                Self::Variant(Variant { ptr_variant: r_ptr, .. })
                | Self::RawVariant(RawVariant { ptr_variant: r_ptr, .. }) if l_ptr == r_ptr,
            ),
            Self::None => matches!(r, Self::None),
        }
    }

    fn effective_kind(&self) -> EffectiveItemKind {
        match self {
            Self::Block(_) => EffectiveItemKind::Block,
            Self::Const(_) | Self::RawConst(_) => EffectiveItemKind::Const,
            Self::Enum(_) | Self::RawEnum(_) => EffectiveItemKind::Enum,
            Self::Field(_) | Self::RawField(_) => EffectiveItemKind::Field,
            Self::Fn(_) | Self::RawFn(_) => EffectiveItemKind::Fn,
            Self::Local(_) | Self::RawLocal(_) => EffectiveItemKind::Local,
            Self::Mod(_) | Self::RawMod(_) => EffectiveItemKind::Mod,
            Self::Struct(_) | Self::RawStruct(_) => EffectiveItemKind::Struct,
            Self::Trait(_) | Self::RawTrait(_) => EffectiveItemKind::Trait,
            Self::TypeAlias(_) | Self::RawTypeAlias(_) => EffectiveItemKind::TypeAlias,
            Self::Use(_) | Self::RawUse(_) => EffectiveItemKind::Use,
            Self::Variant(_) | Self::RawVariant(_) => EffectiveItemKind::Variant,
            Self::None => EffectiveItemKind::Extra,
        }
    }

    fn vis_node(&self) -> TriOption<NodeIndex, ()> {
        match self {
            Self::Const(v) => TriOption::Some(v.vis_node()),
            Self::Enum(Enum { vis_node, .. })
            | Self::Field(Field { vis_node, .. })
            | Self::Fn(Fn { vis_node, .. })
            | Self::Mod(Mod { vis_node, .. })
            | Self::Struct(Struct { vis_node, .. })
            | Self::Trait(Trait { vis_node, .. })
            | Self::TypeAlias(TypeAlias { vis_node, .. })
            | Self::Use(Use { vis_node, .. })
            | Self::Variant(Variant { vis_node, .. }) => TriOption::Some(*vis_node),

            Self::RawConst(v) => v
                .vis_node()
                .map(TriOption::Some)
                .unwrap_or(TriOption::NotYet(())),
            Self::RawEnum(RawEnum { vis_node, .. })
            | Self::RawField(RawField { vis_node, .. })
            | Self::RawFn(RawFn { vis_node, .. })
            | Self::RawMod(RawMod { vis_node, .. })
            | Self::RawStruct(RawStruct { vis_node, .. })
            | Self::RawTrait(RawTrait { vis_node, .. })
            | Self::RawTypeAlias(RawTypeAlias { vis_node, .. })
            | Self::RawUse(RawUse { vis_node, .. })
            | Self::RawVariant(RawVariant { vis_node, .. }) => (*vis_node)
                .map(TriOption::Some)
                .unwrap_or(TriOption::NotYet(())),

            Self::Block(_) | Self::Local(_) | Self::RawLocal(_) | Self::None => TriOption::None,
        }
    }

    fn type_id(&self) -> TriOption<TypeId, ()> {
        match self {
            Self::Const(v) => TriOption::Some(v.type_id()),
            Self::Enum(Enum { tid, .. })
            | Self::Field(Field { tid, .. })
            | Self::Fn(Fn { tid, .. })
            | Self::Local(Local { tid, .. })
            | Self::Struct(Struct { tid, .. })
            | Self::TypeAlias(TypeAlias { tid, .. })
            | Self::Variant(Variant { tid, .. }) => TriOption::Some(*tid),

            // Some raw items will have their types when they are resolved.
            Self::RawConst(v) => v
                .type_id()
                .map(TriOption::Some)
                .unwrap_or(TriOption::NotYet(())),
            Self::RawEnum(_)
            | Self::RawField(_)
            | Self::RawFn(_)
            | Self::RawLocal(_)
            | Self::RawStruct(_)
            | Self::RawTypeAlias(_)
            | Self::RawVariant(_) => TriOption::NotYet(()),

            Self::Block(_)
            | Self::Mod(_)
            | Self::Trait(_)
            | Self::Use(_)
            | Self::RawMod(_)
            | Self::RawTrait(_)
            | Self::RawUse(_)
            | Self::None => TriOption::None,
        }
    }

    fn syn_id(&self) -> Option<SynId> {
        let sid = match self {
            Self::Block(v) => v.syn_id(),
            Self::Const(v) => v.syn_id(),
            Self::Enum(v) => v.syn_id(),
            Self::Field(v) => v.syn_id(),
            Self::Fn(v) => v.syn_id(),
            Self::Local(v) => v.syn_id(),
            Self::Mod(v) => v.syn_id(),
            Self::Struct(v) => v.syn_id(),
            Self::Trait(v) => v.syn_id(),
            Self::TypeAlias(v) => v.syn_id(),
            Self::Use(v) => v.syn_id(),
            Self::Variant(v) => v.syn_id(),
            Self::RawConst(v) => v.syn_id(),
            Self::RawEnum(v) => v.syn_id(),
            Self::RawField(v) => v.syn_id(),
            Self::RawFn(v) => v.syn_id(),
            Self::RawLocal(v) => v.syn_id(),
            Self::RawMod(v) => v.syn_id(),
            Self::RawStruct(v) => v.syn_id(),
            Self::RawTrait(v) => v.syn_id(),
            Self::RawTypeAlias(v) => v.syn_id(),
            Self::RawUse(v) => v.syn_id(),
            Self::RawVariant(v) => v.syn_id(),
            Self::None => return None,
        };
        Some(sid)
    }

    fn as_fn(&self) -> Option<&Fn> {
        if let Self::Fn(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_struct(&self) -> Option<&Struct> {
        if let Self::Struct(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_type_alias(&self) -> Option<&TypeAlias> {
        if let Self::TypeAlias(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_use(&self) -> Option<&Use> {
        if let Self::Use(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_variant(&self) -> Option<&Variant> {
        if let Self::Variant(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_mut_mod(&mut self) -> Option<&mut Mod> {
        if let Self::Mod(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl ItemTrait for PubItem<'_> {
    fn is_effective_same(&self, other: &Self) -> bool {
        mem::discriminant(self) == mem::discriminant(other)
    }

    fn effective_kind(&self) -> EffectiveItemKind {
        match self {
            Self::Block(_) => EffectiveItemKind::Block,
            Self::Const(_) => EffectiveItemKind::Const,
            Self::Enum(_) => EffectiveItemKind::Enum,
            Self::Field(_) => EffectiveItemKind::Field,
            Self::Fn(_) => EffectiveItemKind::Fn,
            Self::Local(_) => EffectiveItemKind::Local,
            Self::Mod(_) => EffectiveItemKind::Mod,
            Self::Struct(_) => EffectiveItemKind::Struct,
            Self::Trait(_) => EffectiveItemKind::Trait,
            Self::TypeAlias(_) => EffectiveItemKind::TypeAlias,
            Self::Use(_) => EffectiveItemKind::Use,
            Self::Variant(_) => EffectiveItemKind::Variant,
        }
    }

    fn vis_node(&self) -> TriOption<NodeIndex, ()> {
        match self {
            Self::Const(v) => TriOption::Some(v.vis_node()),
            Self::Enum(Enum { vis_node, .. })
            | Self::Field(Field { vis_node, .. })
            | Self::Fn(Fn { vis_node, .. })
            | Self::Mod(Mod { vis_node, .. })
            | Self::Struct(Struct { vis_node, .. })
            | Self::Trait(Trait { vis_node, .. })
            | Self::TypeAlias(TypeAlias { vis_node, .. })
            | Self::Use(Use { vis_node, .. })
            | Self::Variant(Variant { vis_node, .. }) => TriOption::Some(*vis_node),

            Self::Block(_) | Self::Local(_) => TriOption::None,
        }
    }

    fn type_id(&self) -> TriOption<TypeId, ()> {
        match self {
            Self::Const(v) => TriOption::Some(v.type_id()),
            Self::Enum(Enum { tid, .. })
            | Self::Field(Field { tid, .. })
            | Self::Fn(Fn { tid, .. })
            | Self::Struct(Struct { tid, .. })
            | Self::TypeAlias(TypeAlias { tid, .. })
            | Self::Local(Local { tid, .. })
            | Self::Variant(Variant { tid, .. }) => TriOption::Some(*tid),

            Self::Block(_) | Self::Mod(_) | Self::Trait(_) | Self::Use(_) => TriOption::None,
        }
    }

    fn syn_id(&self) -> Option<SynId> {
        let sid = match self {
            Self::Block(v) => v.syn_id(),
            Self::Const(v) => v.syn_id(),
            Self::Enum(v) => v.syn_id(),
            Self::Field(v) => v.syn_id(),
            Self::Fn(v) => v.syn_id(),
            Self::Local(v) => v.syn_id(),
            Self::Mod(v) => v.syn_id(),
            Self::Struct(v) => v.syn_id(),
            Self::Trait(v) => v.syn_id(),
            Self::TypeAlias(v) => v.syn_id(),
            Self::Use(v) => v.syn_id(),
            Self::Variant(v) => v.syn_id(),
        };
        Some(sid)
    }

    fn as_fn(&self) -> Option<&Fn> {
        if let Self::Fn(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_struct(&self) -> Option<&Struct> {
        if let Self::Struct(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_type_alias(&self) -> Option<&TypeAlias> {
        if let Self::TypeAlias(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_use(&self) -> Option<&Use> {
        if let Self::Use(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_variant(&self) -> Option<&Variant> {
        if let Self::Variant(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn as_mut_mod(&mut self) -> Option<&mut Mod> {
        None // Read-only
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum EffectiveItemKind {
    Block,
    Const,
    Enum,
    Field,
    Fn,
    Local,
    Mod,
    Struct,
    Trait,
    TypeAlias,
    Use,
    Variant,
    Extra,
}
