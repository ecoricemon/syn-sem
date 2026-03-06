//! Term definitions.

use crate::{semantic::entry::GlobalCx, TermIn};
use logic_eval::{Name, Term};

use crate::etc::util::IntoPathSegments;

// '#' is a prefix to distinguish between reserved functors and user functors.
pub(crate) const FUNCTOR_IMPL: &str = "#impl";
pub(crate) const FUNCTOR_TRAIT: &str = "#trait";
pub(crate) const FUNCTOR_ASSOC_TY: &str = "#assoc_ty";
pub(crate) const FUNCTOR_ASSOC_FN: &str = "#assoc_fn";
pub(crate) const FUNCTOR_INHER_FN: &str = "#inher_fn";
pub(crate) const FUNCTOR_ASSOC_CONST_TY: &str = "#assoc_const_ty";
pub(crate) const FUNCTOR_ASSOC_CONST_VAL: &str = "#assoc_const_val";
pub(crate) const FUNCTOR_INHER_CONST: &str = "#inher_const";
pub(crate) const FUNCTOR_REF: &str = "#ref";
pub(crate) const FUNCTOR_MUT: &str = "#mut";
pub(crate) const FUNCTOR_ARRAY: &str = "#array";
pub(crate) const FUNCTOR_TUPLE: &str = "#tuple";
pub(crate) const FUNCTOR_SIG: &str = "#sig";
pub(crate) const FUNCTOR_INT: &str = "#int";
pub(crate) const FUNCTOR_FLOAT: &str = "#float";
pub(crate) const FUNCTOR_UNIT: &str = "#unit";
pub(crate) const FUNCTOR_LIST: &str = "#list"; // 'logic-eval' doesn't have array('[]') yet.
pub(crate) const FUNCTOR_ARG: &str = "#arg";
pub(crate) const FUNCTOR_DYN_ARRAY_LEN: &str = "#dyn";

// === impl/n ===

/// `impl/1`
///
/// * Arg0 - Self type
///
/// # Examples
///
/// * Code   - impl Foo { .. }
/// * Output - impl(Foo)
pub fn impl_1<'gcx>(self_ty: TermIn<'gcx>, gcx: &'gcx GlobalCx<'gcx>) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_IMPL, gcx),
        args: [self_ty].into(),
    }
}

/// `impl/2`
///
/// * Arg0 - Self type
/// * Arg1 - Trait
///
/// # Examples
///
/// * Code   - impl Trait for Foo { .. }
/// * Output - impl(Foo, Trait)
pub fn impl_2<'gcx>(
    self_ty: TermIn<'gcx>,
    trait_: TermIn<'gcx>,
    gcx: &'gcx GlobalCx<'gcx>,
) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_IMPL, gcx),
        args: [self_ty, trait_].into(),
    }
}

// === trait/n ===

/// `trait/1`
///
/// * Arg0 - Trait
///
/// # Examples
///
/// * Code   - trait Trait { .. }
/// * Output - trait(Trait)
pub fn trait_1<'gcx>(trait_: TermIn<'gcx>, gcx: &'gcx GlobalCx<'gcx>) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_TRAIT, gcx),
        args: [trait_].into(),
    }
}

// === assoc_ty/n ===

/// `assoc_ty/4`
///
/// * Arg0 - Self type
/// * Arg1 - Trait
/// * Arg2 - Associated type
/// * Arg3 - Assigned(Bound) type
///
/// # Examples
///
/// * Code   - impl Iterator for Foo { type Item = Bar; }
/// * Output - assoc_ty(Foo, Iterator, Item, Bar)
pub fn assoc_ty_4<'gcx>(
    self_ty: TermIn<'gcx>,
    trait_: TermIn<'gcx>,
    assoc_ty: TermIn<'gcx>,
    assign_ty: TermIn<'gcx>,
    gcx: &'gcx GlobalCx<'gcx>,
) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_ASSOC_TY, gcx),
        args: [self_ty, trait_, assoc_ty, assign_ty].into(),
    }
}

// === assoc_fn/n ===

/// `assoc_fn/3`
///
/// * Arg0 - Trait
/// * Arg1 - Associated function name
/// * Arg2 - Function signature
///
/// # Examples
///
/// * Code   - trait Trait { fn foo<T>(a: In0, b: In1) -> Out }
/// * Output - assoc_fn(Trait, foo(T), sig(Out, In0, In1))
pub fn assoc_fn_3<'gcx>(
    trait_: TermIn<'gcx>,
    fn_ident: TermIn<'gcx>,
    sig: TermIn<'gcx>,
    gcx: &'gcx GlobalCx<'gcx>,
) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_ASSOC_FN, gcx),
        args: [trait_, fn_ident, sig].into(),
    }
}

// === inher_fn/n ===

/// `inher_fn/3`
///
/// * Arg0 - Self type
/// * Arg1 - Inherent function name
/// * Arg2 - Function signature
///
/// # Examples
///
/// * Code   - impl St { fn foo<T>(a: In0, b: In1) -> Out }
/// * Output - inher_fn(St, foo(T), sig(Out, In0, In1))
pub fn inher_fn_3<'gcx>(
    self_ty: TermIn<'gcx>,
    fn_ident: TermIn<'gcx>,
    sig: TermIn<'gcx>,
    gcx: &'gcx GlobalCx<'gcx>,
) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_INHER_FN, gcx),
        args: [self_ty, fn_ident, sig].into(),
    }
}

// === assoc_const_ty/n ===

/// `assoc_const_ty/3`
///
/// * Arg0 - Trait
/// * Arg1 - Associated const name
/// * Arg2 - Type of the const
///
/// # Examples
///
/// * Code   - trait Trait { const A: ConstTy = .. }
/// * Output - assoc_const_ty(Trait, A, ConstTy)
pub fn assoc_const_ty_3<'gcx>(
    trait_: TermIn<'gcx>,
    const_ident: TermIn<'gcx>,
    const_ty: TermIn<'gcx>,
    gcx: &'gcx GlobalCx<'gcx>,
) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_ASSOC_CONST_TY, gcx),
        args: [trait_, const_ident, const_ty].into(),
    }
}

// === assoc_const_val/n ===

/// `assoc_const_val/3`
///
/// * Arg0 - Trait
/// * Arg1 - Associated const name
/// * Arg2 - Const id
///
/// # Examples
///
/// * Code   - trait Trait { const A: ConstTy = Value }
/// * Output - assoc_const_val(Trait, A, id to the Value)
pub fn assoc_const_val_3<'gcx>(
    trait_: TermIn<'gcx>,
    const_ident: TermIn<'gcx>,
    const_id: TermIn<'gcx>,
    gcx: &'gcx GlobalCx<'gcx>,
) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_ASSOC_CONST_VAL, gcx),
        args: [trait_, const_ident, const_id].into(),
    }
}

/// `assoc_const_val/4`
///
/// * Arg0 - Self type
/// * Arg1 - Trait
/// * Arg2 - Associated const name
/// * Arg3 - Const id
///
/// # Examples
///
/// * Code   - impl Trait for Foo { const A: ConstTy = Value }
/// * Output - assoc_const_val(Foo, Trait, A, id to the Value)
pub fn assoc_const_val_4<'gcx>(
    self_ty: TermIn<'gcx>,
    trait_: TermIn<'gcx>,
    const_ident: TermIn<'gcx>,
    const_id: TermIn<'gcx>,
    gcx: &'gcx GlobalCx<'gcx>,
) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_ASSOC_CONST_VAL, gcx),
        args: [self_ty, trait_, const_ident, const_id].into(),
    }
}

// === inher_const/n ===

/// `inher_const/4`
///
/// * Arg0 - Self type
/// * Arg1 - Associated const name
/// * Arg2 - Type of the const
/// * Arg3 - Const id
///
/// # Examples
///
/// * Code   - impl Foo { const A: ConstTy = Value }
/// * Output - inher_const(Foo, A, ConstTy, id to the Value)
pub fn inher_const_4<'gcx>(
    self_ty: TermIn<'gcx>,
    const_ident: TermIn<'gcx>,
    const_ty: TermIn<'gcx>,
    const_id: TermIn<'gcx>,
    gcx: &'gcx GlobalCx<'gcx>,
) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_INHER_CONST, gcx),
        args: [self_ty, const_ident, const_ty, const_id].into(),
    }
}

// === ref/n ===

/// `ref/1`
///
/// * Arg0 - Type
///
/// # Examples
///
/// * Code   - &i32
/// * Output - ref(i32)
pub fn ref_1<'gcx>(ty: TermIn<'gcx>, gcx: &'gcx GlobalCx<'gcx>) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_REF, gcx),
        args: [ty].into(),
    }
}

// === mut/n ===

/// `mut/1`
///
/// * Arg0 - Type
///
/// # Examples
///
/// * Code   - &mut i32
/// * Output - ref(mut(i32))
pub fn mut_1<'gcx>(ty: TermIn<'gcx>, gcx: &'gcx GlobalCx<'gcx>) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_MUT, gcx),
        args: [ty].into(),
    }
}

// === array/n ===

/// `array/1`
///
/// * Arg0 - Element type
///
/// # Examples
///
/// * Code   - \[i32\]
/// * Output - array(i32, dyn)
pub fn array_1<'gcx>(elem: TermIn<'gcx>, gcx: &'gcx GlobalCx<'gcx>) -> TermIn<'gcx> {
    let len = Term {
        functor: Name::with_intern(FUNCTOR_DYN_ARRAY_LEN, gcx),
        args: [].into(),
    };
    Term {
        functor: Name::with_intern(FUNCTOR_ARRAY, gcx),
        args: [elem, len].into(),
    }
}

/// `array/2`
///
/// * Arg0 - Element type
/// * Arg1 - Length
///
/// # Examples
///
/// * Code   - \[i32; 2\]
/// * Output - array(i32, 2)
pub fn array_2<'gcx>(
    elem: TermIn<'gcx>,
    len: TermIn<'gcx>,
    gcx: &'gcx GlobalCx<'gcx>,
) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_ARRAY, gcx),
        args: [elem, len].into(),
    }
}

// === tuple/n ===

/// `tuple/n`
///
/// * Args - Element types
///
/// # Examples
///
/// * Code   - (A, B)
/// * Output - tuple(A, B)
pub fn tuple_n<'gcx>(elems: Vec<TermIn<'gcx>>, gcx: &'gcx GlobalCx<'gcx>) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_TUPLE, gcx),
        args: elems,
    }
}

// === sig/n ===

/// `sig/n`
///
/// Creates a term representing a function signature.
///
/// * Args - Function parameters (Out, In0, In1, ...)
///
/// # Examples
///
/// * Code   - (a: In0, b: In1) -> Out
/// * Output - sig(Out, In0, In1)
pub fn sig_n<'gcx>(args: Vec<TermIn<'gcx>>, gcx: &'gcx GlobalCx<'gcx>) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_SIG, gcx),
        args,
    }
}

// === int/n ===

/// `int/1`
///
/// * Arg0 - Integer type such as 'i32'
///
/// # Examples
///
/// * Code   - i32
/// * Output - int(i32)
pub fn int_1<'gcx>(int: TermIn<'gcx>, gcx: &'gcx GlobalCx<'gcx>) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_INT, gcx),
        args: [int].into(),
    }
}

// === float/n ===

/// `float/1`
///
/// * Arg0 = Floating type such as 'f32'
///
/// # Examples
///
/// * Code   - f32
/// * Output - float(f32)
pub fn float_1<'gcx>(float: TermIn<'gcx>, gcx: &'gcx GlobalCx<'gcx>) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_FLOAT, gcx),
        args: [float].into(),
    }
}

// === unit/n ===

/// `unit/0`
///
/// Type ()
pub fn unit_0<'gcx>(gcx: &'gcx GlobalCx<'gcx>) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_UNIT, gcx),
        args: [].into(),
    }
}

// === list/n ===

/// `list/n`
///
/// Creates a term that wraps the given arguments in.
///
/// * Args - Elements
///
/// # Examples
///
/// * Code   - a::B::<x::Y>::C
/// * Output - list(a, B(list(x, Y)), C)
pub fn list_n<'gcx>(elems: Vec<TermIn<'gcx>>, gcx: &'gcx GlobalCx<'gcx>) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_LIST, gcx),
        args: elems,
    }
}

// === arg/n ===

/// `arg/n`
///
/// This allows us to make queries for finding all clauses that share the same functor but not
/// arity. For example, we can find 'Foo(T)', 'Foo(T, U)' using just one 'Foo(X)' by introducing the
/// 'arg'.
///
/// * Args - Anonymous number of arguments
///
/// # Examples
///
/// * Code   - Foo<T, U>
/// * Output - Foo(arg(T, U))
pub fn arg_n<'gcx>(args: Vec<TermIn<'gcx>>, gcx: &'gcx GlobalCx<'gcx>) -> TermIn<'gcx> {
    Term {
        functor: Name::with_intern(FUNCTOR_ARG, gcx),
        args,
    }
}

/// # Examples
///
/// * path - "a::b::C"
/// * Output - list(a(arg), b(arg), C(arg))
pub fn path_to_list<'gcx, T: IntoPathSegments>(path: T, gcx: &'gcx GlobalCx<'gcx>) -> TermIn<'gcx> {
    let segments = path
        .segments()
        .map(|segment| {
            let empty_arg = arg_n([].into(), gcx);
            let functor = Name::with_intern(segment.as_ref(), gcx);
            Term {
                functor,
                args: [empty_arg].into(),
            }
        })
        .collect();

    list_n(segments, gcx)
}
