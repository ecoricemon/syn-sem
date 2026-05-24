/// Implements traits in [`IdentifySyn`], [`InsertRelation`], and [`FindChild`] for all syn items.
///
/// [`IdentifySyn`]: crate::IdentifySyn
/// [`InsertRelation`]: crate::InsertRelation
/// [`FindChild`]: crate::FindChild
mod impls {
    use crate::{AsElements, Elements, FindChild, IdentifySyn, InsertRelation, Node, ParentFinder};
    use std::{any::Any, iter};

    macro_rules! impl_identify_syn {
        ($ty:ty) => {
            impl IdentifySyn for $ty {
                fn as_any(&self) -> &dyn Any {
                    self
                }

                fn type_name(&self) -> &'static str {
                    std::any::type_name::<$ty>()
                }
            }
        };
    }

    macro_rules! impl_as_elements {
        ($ty:ty) => {
            impl AsElements for $ty {
                type Output<'a>
                    = Elements<iter::Once<Node>>
                where
                    Self: 'a;

                fn as_elements(&self) -> Self::Output<'_> {
                    let node = Node::from(self);
                    Elements::Iter(iter::once(node))
                }
            }
        };
    }

    macro_rules! impl_insert_relation_simple {
        ($ty:ty $(, $($fields:ident),* )?) => {
            impl InsertRelation for $ty {
                #[allow(unused_variables)]
                fn insert_relation(&self, finder: &mut ParentFinder) {
                    let parent = self.syn_id();
                    $($(
                        for child in self.$fields.as_elements() {
                            let child: Node = child;
                            finder.insert(child.syn_id(), parent);
                            child.as_dyn_insert_relation()
                                .insert_relation(finder);
                        }
                    )*)?
                }
            }
        };
    }

    macro_rules! impl_insert_relation_match {
        (
            $ty:ty
            $(, $( $arms:pat => { $($fields:ident),* } )* )?
        ) =>
        {
            impl InsertRelation for $ty {
                #[allow(unreachable_patterns, unused_variables)]
                fn insert_relation(&self, finder: &mut ParentFinder) {
                    let parent = self.syn_id();
                    match self {
                        $($(
                            $arms => {
                                $(
                                    finder.insert($fields.syn_id(), parent);
                                    $fields.insert_relation(finder);
                                )*
                            }
                        )*)?
                        _ => {}
                    }
                }
            }
        };
    }

    macro_rules! impl_find_child_simple {
        ($ty:ty $(, $($fields:ident),* )?) => {
            impl FindChild for $ty {
                fn visit_descendant<F: FnMut(usize, crate::identify::SynId)>(
                    &self,
                    descendant_types: &[std::any::TypeId],
                    f: &mut F
                ) {
                    if let Some((i, _)) = descendant_types
                        .iter()
                        .enumerate()
                        .find(|(_, descendant)| **descendant == std::any::TypeId::of::<Self>())
                    {
                        f(i, self.syn_id());
                    }

                    $($(
                        self.$fields.visit_descendant(descendant_types, f);
                    )*)?
                }
            }
        };
    }

    macro_rules! impl_find_child_match {
        (
            $ty:ty
            $(, $( $arms:pat => { $($fields:ident),* } )* )?
        ) =>
        {
            impl FindChild for $ty {
                #[allow(unreachable_patterns, unused_variables)]
                fn visit_descendant<F: FnMut(usize, crate::identify::SynId)>(
                    &self,
                    descendant_types: &[std::any::TypeId],
                    f: &mut F,
                ) {
                    if let Some((i, _)) = descendant_types
                        .iter()
                        .enumerate()
                        .find(|(_, descendant)| **descendant == std::any::TypeId::of::<Self>())
                    {
                        f(i, self.syn_id());
                    }

                    match self {
                        $($(
                            $arms => {
                                $(
                                    $fields.visit_descendant(descendant_types, f);
                                )*
                            }
                        )*)?
                        _ => {}
                    }
                }
            }
        };
    }

    macro_rules! impl_all_simple {
        ($ty:ty $(, $($fields:ident),* )?) => {
            impl_identify_syn!($ty);
            impl_as_elements!($ty);
            impl_insert_relation_simple!($ty $(, $($fields),* )?);
            impl_find_child_simple!($ty $(, $($fields),* )?);
        };
    }

    macro_rules! impl_all_match {
        (
            $ty:ty
            $(, $( $arms:pat => { $($fields:ident),* } )* )?) =>
        {
            impl_identify_syn!($ty);
            impl_as_elements!($ty);
            impl_insert_relation_match!($ty $(,$( $arms => { $($fields),* } )*)?);
            impl_find_child_match!($ty $(,$( $arms => { $($fields),* } )*)?);
        };
    }

    // === Implement for this crate ===

    // === Implement for syn ===

    impl_all_simple!(syn::Token![abstract]);
    impl_all_simple!(syn::Token![as]);
    impl_all_simple!(syn::Token![async]);
    impl_all_simple!(syn::Token![auto]);
    impl_all_simple!(syn::Token![await]);
    impl_all_simple!(syn::Token![become]);
    impl_all_simple!(syn::Token![box]);
    impl_all_simple!(syn::Token![break]);
    impl_all_simple!(syn::Token![const]);
    impl_all_simple!(syn::Token![continue]);
    impl_all_simple!(syn::Token![crate]);
    impl_all_simple!(syn::Token![default]);
    impl_all_simple!(syn::Token![do]);
    impl_all_simple!(syn::Token![dyn]);
    impl_all_simple!(syn::Token![else]);
    impl_all_simple!(syn::Token![enum]);
    impl_all_simple!(syn::Token![extern]);
    impl_all_simple!(syn::Token![final]);
    impl_all_simple!(syn::Token![fn]);
    impl_all_simple!(syn::Token![for]);
    impl_all_simple!(syn::Token![if]);
    impl_all_simple!(syn::Token![impl]);
    impl_all_simple!(syn::Token![in]);
    impl_all_simple!(syn::Token![let]);
    impl_all_simple!(syn::Token![loop]);
    impl_all_simple!(syn::Token![macro]);
    impl_all_simple!(syn::Token![match]);
    impl_all_simple!(syn::Token![mod]);
    impl_all_simple!(syn::Token![move]);
    impl_all_simple!(syn::Token![mut]);
    impl_all_simple!(syn::Token![override]);
    impl_all_simple!(syn::Token![priv]);
    impl_all_simple!(syn::Token![pub]);
    impl_all_simple!(syn::Token![raw]);
    impl_all_simple!(syn::Token![ref]);
    impl_all_simple!(syn::Token![return]);
    impl_all_simple!(syn::Token![Self]);
    impl_all_simple!(syn::Token![self]);
    impl_all_simple!(syn::Token![static]);
    impl_all_simple!(syn::Token![struct]);
    impl_all_simple!(syn::Token![super]);
    impl_all_simple!(syn::Token![trait]);
    impl_all_simple!(syn::Token![try]);
    impl_all_simple!(syn::Token![type]);
    impl_all_simple!(syn::Token![typeof]);
    impl_all_simple!(syn::Token![union]);
    impl_all_simple!(syn::Token![unsafe]);
    impl_all_simple!(syn::Token![unsized]);
    impl_all_simple!(syn::Token![use]);
    impl_all_simple!(syn::Token![virtual]);
    impl_all_simple!(syn::Token![where]);
    impl_all_simple!(syn::Token![while]);
    impl_all_simple!(syn::Token![yield]);
    impl_all_simple!(syn::Token![&]);
    impl_all_simple!(syn::Token![&&]);
    impl_all_simple!(syn::Token![&=]);
    impl_all_simple!(syn::Token![@]);
    impl_all_simple!(syn::Token![^]);
    impl_all_simple!(syn::Token![^=]);
    impl_all_simple!(syn::Token![:]);
    impl_all_simple!(syn::Token![,]);
    impl_all_simple!(syn::Token![$]);
    impl_all_simple!(syn::Token![.]);
    impl_all_simple!(syn::Token![..]);
    impl_all_simple!(syn::Token![...]);
    impl_all_simple!(syn::Token![..=]);
    impl_all_simple!(syn::Token![=]);
    impl_all_simple!(syn::Token![==]);
    impl_all_simple!(syn::Token![=>]);
    impl_all_simple!(syn::Token![>=]);
    impl_all_simple!(syn::Token![>]);
    impl_all_simple!(syn::Token![<-]);
    impl_all_simple!(syn::Token![<=]);
    impl_all_simple!(syn::Token![<]);
    impl_all_simple!(syn::Token![-]);
    impl_all_simple!(syn::Token![-=]);
    impl_all_simple!(syn::Token![!=]);
    impl_all_simple!(syn::Token![!]);
    impl_all_simple!(syn::Token![|]);
    impl_all_simple!(syn::Token![|=]);
    impl_all_simple!(syn::Token![||]);
    impl_all_simple!(syn::Token![::]);
    impl_all_simple!(syn::Token![%]);
    impl_all_simple!(syn::Token![%=]);
    impl_all_simple!(syn::Token![+]);
    impl_all_simple!(syn::Token![+=]);
    impl_all_simple!(syn::Token![#]);
    impl_all_simple!(syn::Token![?]);
    impl_all_simple!(syn::Token![->]);
    impl_all_simple!(syn::Token![;]);
    impl_all_simple!(syn::Token![<<]);
    impl_all_simple!(syn::Token![<<=]);
    impl_all_simple!(syn::Token![>>]);
    impl_all_simple!(syn::Token![>>=]);
    impl_all_simple!(syn::Token![/]);
    impl_all_simple!(syn::Token![/=]);
    impl_all_simple!(syn::Token![*]);
    impl_all_simple!(syn::Token![*=]);
    impl_all_simple!(syn::Token![~]);
    impl_all_simple!(syn::Token![_]);
    impl_all_simple!(syn::token::Group);
    impl_all_simple!(syn::token::Brace);
    impl_all_simple!(syn::token::Bracket);
    impl_all_simple!(syn::token::Paren);
    impl_all_simple!(syn::Abi, extern_token, name);
    impl_all_simple!(
        syn::AngleBracketedGenericArguments,
        colon2_token,
        lt_token,
        args,
        gt_token
    );
    impl_all_simple!(syn::Arm, attrs, pat, guard, fat_arrow_token, body, comma);
    impl_all_simple!(syn::AssocConst, ident, generics, eq_token, value);
    impl_all_simple!(syn::AssocType, ident, generics, eq_token, ty);
    impl_all_simple!(syn::Attribute, pound_token, style, bracket_token, meta);
    impl_all_match!(
        syn::AttrStyle,
        Self::Outer => {}
        Self::Inner(v) => {v}
    );
    impl_all_simple!(syn::BareFnArg, attrs, name, ty);
    impl_all_simple!(syn::BareVariadic, attrs, name, dots, comma);
    impl_all_match!(
        syn::BinOp,
        Self::Add(v) => {v}
        Self::Sub(v) => {v}
        Self::Mul(v) => {v}
        Self::Div(v) => {v}
        Self::Rem(v) => {v}
        Self::And(v) => {v}
        Self::Or(v) => {v}
        Self::BitXor(v) => {v}
        Self::BitAnd(v) => {v}
        Self::BitOr(v) => {v}
        Self::Shl(v) => {v}
        Self::Shr(v) => {v}
        Self::Eq(v) => {v}
        Self::Lt(v) => {v}
        Self::Le(v) => {v}
        Self::Ne(v) => {v}
        Self::Ge(v) => {v}
        Self::Gt(v) => {v}
        Self::AddAssign(v) => {v}
        Self::SubAssign(v) => {v}
        Self::MulAssign(v) => {v}
        Self::DivAssign(v) => {v}
        Self::RemAssign(v) => {v}
        Self::BitXorAssign(v) => {v}
        Self::BitAndAssign(v) => {v}
        Self::BitOrAssign(v) => {v}
        Self::ShlAssign(v) => {v}
        Self::ShrAssign(v) => {v}
    );
    impl_all_simple!(syn::Block, brace_token, stmts);
    impl_all_simple!(
        syn::BoundLifetimes,
        for_token,
        lt_token,
        lifetimes,
        gt_token
    );
    impl_all_match!(
        syn::CapturedParam,
        Self::Lifetime(v) => {v}
        Self::Ident(v) => {v}
    );
    impl_all_simple!(
        syn::ConstParam,
        attrs,
        const_token,
        ident,
        colon_token,
        ty,
        eq_token,
        default
    );
    impl_all_simple!(syn::Constraint, ident, generics, colon_token, bounds);
    impl_all_match!(
        syn::Expr,
        Self::Array(v) => {v}
        Self::Assign(v) => {v}
        Self::Async(v) => {v}
        Self::Await(v) => {v}
        Self::Binary(v) => {v}
        Self::Block(v) => {v}
        Self::Break(v) => {v}
        Self::Call(v) => {v}
        Self::Cast(v) => {v}
        Self::Closure(v) => {v}
        Self::Const(v) => {v}
        Self::Continue(v) => {v}
        Self::Field(v) => {v}
        Self::ForLoop(v) => {v}
        Self::Group(v) => {v}
        Self::If(v) => {v}
        Self::Index(v) => {v}
        Self::Infer(v) => {v}
        Self::Let(v) => {v}
        Self::Lit(v) => {v}
        Self::Loop(v) => {v}
        Self::Macro(v) => {v}
        Self::Match(v) => {v}
        Self::MethodCall(v) => {v}
        Self::Paren(v) => {v}
        Self::Path(v) => {v}
        Self::Range(v) => {v}
        Self::RawAddr(v) => {v}
        Self::Reference(v) => {v}
        Self::Repeat(v) => {v}
        Self::Return(v) => {v}
        Self::Struct(v) => {v}
        Self::Try(v) => {v}
        Self::TryBlock(v) => {v}
        Self::Tuple(v) => {v}
        Self::Unary(v) => {v}
        Self::Unsafe(v) => {v}
        Self::Verbatim(_) => {}
        Self::While(v) => {v}
        Self::Yield(v) => {v}
    );
    impl_all_simple!(syn::ExprArray, attrs, bracket_token, elems);
    impl_all_simple!(syn::ExprAssign, attrs, left, eq_token, right);
    impl_all_simple!(syn::ExprAsync, attrs, async_token, capture, block);
    impl_all_simple!(syn::ExprAwait, attrs, base, dot_token, await_token);
    impl_all_simple!(syn::ExprBinary, attrs, left, op, right);
    impl_all_simple!(syn::ExprBlock, attrs, label, block);
    impl_all_simple!(syn::ExprBreak, attrs, break_token, label, expr);
    impl_all_simple!(syn::ExprCall, attrs, func, paren_token, args);
    impl_all_simple!(syn::ExprCast, attrs, expr, as_token, ty);
    impl_all_simple!(
        syn::ExprClosure,
        attrs,
        lifetimes,
        constness,
        movability,
        asyncness,
        capture,
        or1_token,
        inputs,
        or2_token,
        output,
        body
    );
    impl_all_simple!(syn::ExprConst, attrs, const_token, block);
    impl_all_simple!(syn::ExprContinue, attrs, continue_token, label);
    impl_all_simple!(syn::ExprField, attrs, base, dot_token, member);
    impl_all_simple!(
        syn::ExprForLoop,
        attrs,
        label,
        for_token,
        pat,
        in_token,
        expr,
        body
    );
    impl_all_simple!(syn::ExprGroup, attrs, group_token, expr);
    impl_all_simple!(syn::ExprIf, attrs, if_token, cond, then_branch, else_branch);
    impl_all_simple!(syn::ExprIndex, attrs, expr, bracket_token, index);
    impl_all_simple!(syn::ExprInfer, attrs, underscore_token);
    impl_all_simple!(syn::ExprLet, attrs, let_token, pat, eq_token, expr);
    impl_all_simple!(syn::ExprLit, attrs, lit);
    impl_all_simple!(syn::ExprLoop, attrs, label, loop_token, body);
    impl_all_simple!(syn::ExprMacro, attrs, mac);
    impl_all_simple!(syn::ExprMatch, attrs, match_token, expr, brace_token, arms);
    impl_all_simple!(
        syn::ExprMethodCall,
        attrs,
        receiver,
        dot_token,
        method,
        turbofish,
        paren_token,
        args
    );
    impl_all_simple!(syn::ExprParen, attrs, paren_token, expr);
    impl_all_simple!(syn::ExprPath, attrs, qself, path);
    impl_all_simple!(syn::ExprRange, attrs, start, limits, end);
    impl_all_simple!(syn::ExprRawAddr, attrs, and_token, raw, mutability, expr);
    impl_all_simple!(syn::ExprReference, attrs, and_token, mutability, expr);
    impl_all_simple!(syn::ExprRepeat, attrs, bracket_token, expr, len);
    impl_all_simple!(syn::ExprReturn, attrs, return_token, expr);
    impl_all_simple!(
        syn::ExprStruct,
        attrs,
        qself,
        path,
        brace_token,
        fields,
        dot2_token,
        rest
    );
    impl_all_simple!(syn::ExprTry, attrs, expr, question_token);
    impl_all_simple!(syn::ExprTryBlock, attrs, try_token, block);
    impl_all_simple!(syn::ExprTuple, attrs, paren_token, elems);
    impl_all_simple!(syn::ExprUnary, attrs, op, expr);
    impl_all_simple!(syn::ExprUnsafe, attrs, unsafe_token, block);
    impl_all_simple!(syn::ExprWhile, attrs, label, while_token, cond, body);
    impl_all_simple!(syn::ExprYield, attrs, yield_token, expr);
    impl_all_simple!(syn::Field, attrs, vis, mutability, ident, colon_token, ty);
    impl_all_match!(
        syn::FieldMutability,
        Self::None => {}
    );
    impl_all_simple!(syn::FieldPat, attrs, member, colon_token, pat);
    impl_all_match!(
        syn::Fields,
        Self::Named(v) => {v}
        Self::Unnamed(v) => {v}
        Self::Unit => {}
    );
    impl_all_simple!(syn::FieldsNamed, brace_token, named);
    impl_all_simple!(syn::FieldsUnnamed, paren_token, unnamed);
    impl_all_simple!(syn::FieldValue, attrs, member, colon_token, expr);
    impl_all_simple!(syn::File, /*shebang*/ attrs, items);
    impl_all_match!(
        syn::FnArg,
        Self::Receiver(v) => {v}
        Self::Typed(v) => {v}
    );
    impl_all_match!(
        syn::ForeignItem,
        Self::Fn(v) => {v}
        Self::Static(v) => {v}
        Self::Type(v) => {v}
        Self::Macro(v) => {v}
        Self::Verbatim(_) => {}
    );
    impl_all_simple!(syn::ForeignItemFn, attrs, vis, sig, semi_token);
    impl_all_simple!(
        syn::ForeignItemStatic,
        attrs,
        vis,
        static_token,
        mutability,
        ident,
        colon_token,
        ty,
        semi_token
    );
    impl_all_simple!(
        syn::ForeignItemType,
        attrs,
        vis,
        type_token,
        ident,
        generics,
        semi_token
    );
    impl_all_simple!(syn::ForeignItemMacro, attrs, mac, semi_token);
    impl_all_match!(
        syn::GenericArgument,
        Self::Lifetime(v) => {v}
        Self::Type(v) => {v}
        Self::Const(v) => {v}
        Self::AssocType(v) => {v}
        Self::AssocConst(v) => {v}
        Self::Constraint(v) => {v}
    );
    impl_all_match!(
        syn::GenericParam,
        Self::Lifetime(v) => {v}
        Self::Type(v) => {v}
        Self::Const(v) => {v}
    );
    impl_all_simple!(syn::Generics, lt_token, params, gt_token, where_clause);
    impl_all_simple!(syn::Ident);
    impl_all_match!(
        syn::ImplItem,
        Self::Const(v) => {v}
        Self::Fn(v) => {v}
        Self::Type(v) => {v}
        Self::Macro(v) => {v}
        Self::Verbatim(_) => {}
    );
    impl_all_simple!(
        syn::ImplItemConst,
        attrs,
        vis,
        defaultness,
        const_token,
        ident,
        generics,
        colon_token,
        ty,
        eq_token,
        expr,
        semi_token
    );
    impl_all_simple!(syn::ImplItemFn, attrs, vis, defaultness, sig, block);
    impl_all_simple!(
        syn::ImplItemType,
        attrs,
        vis,
        defaultness,
        type_token,
        ident,
        generics,
        eq_token,
        ty,
        semi_token
    );
    impl_all_simple!(syn::ImplItemMacro, attrs, mac, semi_token);
    impl_all_match!(syn::ImplRestriction);
    impl_all_simple!(syn::Index);
    impl_all_match!(
        syn::Item,
        Self::Const(v) => {v}
        Self::Enum(v) => {v}
        Self::ExternCrate(v) => {v}
        Self::Fn(v) => {v}
        Self::ForeignMod(v) => {v}
        Self::Impl(v) => {v}
        Self::Macro(v) => {v}
        Self::Mod(v) => {v}
        Self::Static(v) => {v}
        Self::Struct(v) => {v}
        Self::Trait(v) => {v}
        Self::TraitAlias(v) => {v}
        Self::Type(v) => {v}
        Self::Union(v) => {v}
        Self::Use(v) => {v}
        Self::Verbatim(_) => {}
    );
    impl_all_simple!(
        syn::ItemConst,
        attrs,
        vis,
        const_token,
        ident,
        generics,
        colon_token,
        ty,
        eq_token,
        expr,
        semi_token
    );
    impl_all_simple!(
        syn::ItemEnum,
        attrs,
        vis,
        enum_token,
        ident,
        generics,
        brace_token,
        variants
    );
    impl_all_simple!(
        syn::ItemExternCrate,
        attrs,
        vis,
        extern_token,
        crate_token,
        ident,
        rename,
        semi_token
    );
    impl_all_simple!(syn::ItemFn, attrs, vis, sig, block);
    impl_all_simple!(
        syn::ItemForeignMod,
        attrs,
        unsafety,
        abi,
        brace_token,
        items
    );
    impl_all_simple!(
        syn::ItemImpl,
        attrs,
        defaultness,
        unsafety,
        impl_token,
        generics,
        trait_,
        self_ty,
        brace_token,
        items
    );
    impl_all_simple!(syn::ItemMacro, attrs, ident, mac, semi_token);
    impl_all_simple!(
        syn::ItemMod,
        attrs,
        vis,
        unsafety,
        mod_token,
        ident,
        content,
        semi
    );
    impl_all_simple!(
        syn::ItemStatic,
        attrs,
        vis,
        static_token,
        mutability,
        ident,
        colon_token,
        ty,
        eq_token,
        expr,
        semi_token
    );
    impl_all_simple!(
        syn::ItemStruct,
        attrs,
        vis,
        struct_token,
        ident,
        generics,
        fields,
        semi_token
    );
    impl_all_simple!(
        syn::ItemTrait,
        attrs,
        vis,
        unsafety,
        auto_token,
        restriction,
        trait_token,
        ident,
        generics,
        colon_token,
        supertraits,
        brace_token,
        items
    );
    impl_all_simple!(
        syn::ItemTraitAlias,
        attrs,
        vis,
        trait_token,
        ident,
        generics,
        eq_token,
        bounds,
        semi_token
    );
    impl_all_simple!(
        syn::ItemType,
        attrs,
        vis,
        type_token,
        ident,
        generics,
        eq_token,
        ty,
        semi_token
    );
    impl_all_simple!(
        syn::ItemUnion,
        attrs,
        vis,
        union_token,
        ident,
        generics,
        fields
    );
    impl_all_simple!(
        syn::ItemUse,
        attrs,
        vis,
        use_token,
        leading_colon,
        tree,
        semi_token
    );
    impl_all_simple!(syn::Label, name, colon_token);
    impl_all_simple!(syn::Lifetime, ident);
    impl_all_simple!(syn::LifetimeParam, attrs, lifetime, colon_token, bounds);
    impl_all_match!(
        syn::Lit,
        Self::Str(v) => {v}
        Self::ByteStr(v) => {v}
        Self::CStr(v) => {v}
        Self::Byte(v) => {v}
        Self::Char(v) => {v}
        Self::Int(v) => {v}
        Self::Float(v) => {v}
        Self::Bool(v) => {v}
        Self::Verbatim(_) => {}
    );
    impl_all_simple!(syn::LitStr);
    impl_all_simple!(syn::LitByteStr);
    impl_all_simple!(syn::LitCStr);
    impl_all_simple!(syn::LitByte);
    impl_all_simple!(syn::LitChar);
    impl_all_simple!(syn::LitInt);
    impl_all_simple!(syn::LitFloat);
    impl_all_simple!(syn::LitBool);
    impl_all_simple!(syn::Local, attrs, let_token, pat, init, semi_token);
    impl_all_simple!(syn::LocalInit, eq_token, expr, diverge);
    impl_all_simple!(syn::Macro, path, bang_token, delimiter);
    impl_all_match!(
        syn::MacroDelimiter,
        Self::Paren(v) => {v}
        Self::Brace(v) => {v}
        Self::Bracket(v) => {v}
    );
    impl_all_match!(
        syn::Member,
        Self::Named(v) => {v}
        Self::Unnamed(v) => {v}
    );
    impl_all_match!(
        syn::Meta,
        Self::Path(v) => {v}
        Self::List(v) => {v}
        Self::NameValue(v) => {v}
    );
    impl_all_simple!(syn::MetaList, path, delimiter);
    impl_all_simple!(syn::MetaNameValue, path, eq_token, value);
    impl_all_simple!(
        syn::ParenthesizedGenericArguments,
        paren_token,
        inputs,
        output
    );
    impl_all_match!(
        syn::Pat,
        Self::Const(v) => {v}
        Self::Ident(v) => {v}
        Self::Lit(v) => {v}
        Self::Macro(v) => {v}
        Self::Or(v) => {v}
        Self::Paren(v) => {v}
        Self::Path(v) => {v}
        Self::Range(v) => {v}
        Self::Reference(v) => {v}
        Self::Rest(v) => {v}
        Self::Slice(v) => {v}
        Self::Struct(v) => {v}
        Self::Tuple(v) => {v}
        Self::TupleStruct(v) => {v}
        Self::Type(v) => {v}
        Self::Verbatim(_) => {}
        Self::Wild(v) => {v}
    );
    impl_all_simple!(syn::PatIdent, attrs, by_ref, mutability, ident, subpat);
    impl_all_simple!(syn::PatOr, attrs, leading_vert, cases);
    impl_all_simple!(syn::PatParen, attrs, paren_token, pat);
    impl_all_simple!(syn::PatReference, attrs, and_token, mutability, pat);
    impl_all_simple!(syn::PatRest, attrs, dot2_token);
    impl_all_simple!(syn::PatSlice, attrs, bracket_token, elems);
    impl_all_simple!(
        syn::PatStruct,
        attrs,
        qself,
        path,
        brace_token,
        fields,
        rest
    );
    impl_all_simple!(syn::PatTuple, attrs, paren_token, elems);
    impl_all_simple!(syn::PatTupleStruct, attrs, qself, path, paren_token, elems);
    impl_all_simple!(syn::PatType, attrs, pat, colon_token, ty);
    impl_all_simple!(syn::PatWild, attrs, underscore_token);
    impl_all_simple!(syn::Path, leading_colon, segments);
    impl_all_match!(
        syn::PathArguments,
        Self::None => {}
        Self::AngleBracketed(v) => {v}
        Self::Parenthesized(v) => {v}
    );
    impl_all_simple!(syn::PathSegment, ident, arguments);
    impl_all_match!(
        syn::PointerMutability,
        Self::Const(v) => {v}
        Self::Mut(v) => {v}
    );
    impl_all_simple!(syn::PreciseCapture, use_token, lt_token, params, gt_token);
    impl_all_simple!(syn::PredicateLifetime, lifetime, colon_token, bounds);
    impl_all_simple!(
        syn::PredicateType,
        lifetimes,
        bounded_ty,
        colon_token,
        bounds
    );
    impl_all_simple!(
        syn::QSelf,
        lt_token,
        ty,
        /*position*/ as_token,
        gt_token
    );
    impl_all_match!(
        syn::RangeLimits,
        Self::HalfOpen(v) => {v}
        Self::Closed(v) => {v}
    );
    impl_all_simple!(
        syn::Receiver,
        attrs,
        reference,
        mutability,
        self_token,
        colon_token,
        ty
    );
    impl_all_match!(
        syn::ReturnType,
        Self::Default => {}
        Self::Type(arrow, ty) => {arrow, ty}
    );
    impl_all_simple!(
        syn::Signature,
        constness,
        asyncness,
        unsafety,
        abi,
        fn_token,
        ident,
        generics,
        paren_token,
        inputs,
        variadic,
        output
    );
    impl_all_match!(
        syn::StaticMutability,
        Self::Mut(v) => {v}
        Self::None => {}
    );
    impl_all_match!(
        syn::Stmt,
        Self::Local(v) => {v}
        Self::Item(v) => {v}
        Self::Expr(expr, Some(semi)) => {expr, semi}
        Self::Expr(expr, None) => {expr}
        Self::Macro(v) => {v}
    );
    impl_all_simple!(syn::StmtMacro, attrs, mac, semi_token);
    impl_all_simple!(syn::TraitBound, paren_token, modifier, lifetimes, path);
    impl_all_match!(
        syn::TraitBoundModifier,
        Self::None => {}
        Self::Maybe(v) => {v}
    );
    impl_all_match!(
        syn::TraitItem,
        Self::Const(v) => {v}
        Self::Fn(v) => {v}
        Self::Type(v) => {v}
        Self::Macro(v) => {v}
        Self::Verbatim(_) => {}
    );
    impl_all_simple!(
        syn::TraitItemConst,
        attrs,
        const_token,
        ident,
        generics,
        colon_token,
        ty,
        default,
        semi_token
    );
    impl_all_simple!(syn::TraitItemFn, attrs, sig, default, semi_token);
    impl_all_simple!(
        syn::TraitItemType,
        attrs,
        type_token,
        ident,
        generics,
        colon_token,
        bounds,
        default,
        semi_token
    );
    impl_all_simple!(syn::TraitItemMacro, attrs, mac, semi_token);
    impl_all_match!(
        syn::Type,
        Self::Array(v) => {v}
        Self::BareFn(v) => {v}
        Self::Group(v) => {v}
        Self::ImplTrait(v) => {v}
        Self::Infer(v) => {v}
        Self::Macro(v) => {v}
        Self::Never(v) => {v}
        Self::Paren(v) => {v}
        Self::Path(v) => {v}
        Self::Ptr(v) => {v}
        Self::Reference(v) => {v}
        Self::Slice(v) => {v}
        Self::TraitObject(v) => {v}
        Self::Tuple(v) => {v}
        Self::Verbatim(_) => {}
    );
    impl_all_simple!(syn::TypeArray, bracket_token, elem, semi_token, len);
    impl_all_simple!(
        syn::TypeBareFn,
        lifetimes,
        unsafety,
        abi,
        fn_token,
        paren_token,
        inputs,
        variadic,
        output
    );
    impl_all_simple!(syn::TypeGroup, group_token, elem);
    impl_all_simple!(syn::TypeImplTrait, impl_token, bounds);
    impl_all_simple!(syn::TypeInfer, underscore_token);
    impl_all_simple!(syn::TypeMacro, mac);
    impl_all_simple!(syn::TypeNever, bang_token);
    impl_all_simple!(syn::TypeParen, paren_token, elem);
    impl_all_simple!(syn::TypePath, qself, path);
    impl_all_simple!(syn::TypePtr, star_token, const_token, mutability, elem);
    impl_all_simple!(syn::TypeReference, and_token, lifetime, mutability, elem);
    impl_all_simple!(syn::TypeSlice, bracket_token, elem);
    impl_all_simple!(syn::TypeTraitObject, dyn_token, bounds);
    impl_all_simple!(syn::TypeTuple, paren_token, elems);
    impl_all_simple!(
        syn::TypeParam,
        attrs,
        ident,
        colon_token,
        bounds,
        eq_token,
        default
    );
    impl_all_match!(
        syn::TypeParamBound,
        Self::Trait(v) => {v}
        Self::Lifetime(v) => {v}
        Self::PreciseCapture(v) => {v}
        Self::Verbatim(_) => {}
    );
    impl_all_match!(
        syn::UnOp,
        Self::Deref(v) => {v}
        Self::Not(v) => {v}
        Self::Neg(v) => {v}
    );
    impl_all_simple!(syn::UseGlob, star_token);
    impl_all_simple!(syn::UseGroup, brace_token, items);
    impl_all_simple!(syn::UseName, ident);
    impl_all_simple!(syn::UsePath, ident, colon2_token, tree);
    impl_all_simple!(syn::UseRename, ident, as_token, rename);
    impl_all_match!(
        syn::UseTree,
        Self::Path(v) => {v}
        Self::Name(v) => {v}
        Self::Rename(v) => {v}
        Self::Glob(v) => {v}
        Self::Group(v) => {v}
    );
    impl_all_simple!(syn::Variadic, attrs, pat, dots, comma);
    impl_all_simple!(syn::Variant, attrs, ident, fields, discriminant);
    impl_all_match!(
        syn::Visibility,
        Self::Public(v) => {v}
        Self::Restricted(v) => {v}
        Self::Inherited => {}
    );
    impl_all_simple!(syn::VisRestricted, pub_token, paren_token, in_token, path);
    impl_all_simple!(syn::WhereClause, where_token, predicates);
    impl_all_match!(
        syn::WherePredicate,
        Self::Lifetime(v) => {v}
        Self::Type(v) => {v}
    );
}
