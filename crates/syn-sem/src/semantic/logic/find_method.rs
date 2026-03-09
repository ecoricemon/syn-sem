use crate::{
    err,
    etc::util::IntoPathSegments,
    semantic::{
        entry::GlobalCx,
        logic::{term, util::var_name, ImplLogic},
    },
    ExprIn, TermIn, TriResult,
};
use logic_eval::{Expr, Name, Term, VAR_PREFIX};

pub(crate) trait Host<'gcx> {
    fn is_visible(&mut self, parent_path: &str, fn_ident: &str) -> TriResult<bool, ()>;
}

#[macro_export]
macro_rules! impl_empty_method_host {
    ($ty:ty) => {
        impl<'gcx> $crate::semantic::logic::find_method::Host<'gcx> for $ty {
            fn is_visible(
                &mut self,
                _parent_path: &str,
                _fn_ident: &str,
            ) -> $crate::TriResult<bool, ()> {
                Ok(true)
            }
        }
    };
}

pub(crate) struct MethodFinder<'a, 'gcx, H> {
    gcx: &'gcx GlobalCx<'gcx>,
    impl_logic: &'a mut ImplLogic<'gcx>,
    host: &'a mut H,
}

impl<'a, 'gcx, H: Host<'gcx>> MethodFinder<'a, 'gcx, H>
where
    'gcx: 'a,
    H: 'a,
{
    const CONCRETE_INTS: [&'static str; 12] = [
        "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
    ];
    const CONCRETE_FLOATS: [&'static str; 2] = ["f32", "f64"];

    pub(crate) fn new(
        gcx: &'gcx GlobalCx<'gcx>,
        impl_logic: &'a mut ImplLogic<'gcx>,
        host: &'a mut H,
    ) -> Self {
        Self {
            gcx,
            impl_logic,
            host,
        }
    }

    /// Finds matching method signature, then puts it in the given `args`.
    ///
    /// * parent_path - The path of a trait or type
    /// * fn_ident - Function name
    /// * args - Actual types of values that are being passed to the function.
    pub(crate) fn find_method_sig(
        &mut self,
        parent_path: &str,
        fn_ident: &str,
        args: &mut [TermIn<'gcx>],
    ) -> TriResult<(), ()> {
        const PARENT_VAR: &str = "$P";
        const FN_NAME_VAR: &str = "$F";
        const _: () = assert!(VAR_PREFIX == '$');

        // Parent(trait or type) term
        let parent_var = Term {
            functor: Name::with_intern(PARENT_VAR, self.gcx),
            args: [].into(),
        };
        let n = parent_path.segments().count();
        let parent = match n as u32 {
            0 => unreachable!(),
            1 => Term {
                functor: Name::with_intern(parent_path, self.gcx),
                args: [parent_var].into(),
            },
            2.. => {
                let empty_arg = || term::arg_n([].into(), self.gcx);
                let mut segments: Vec<TermIn<'gcx>> = parent_path
                    .segments()
                    .map(|segment| Term {
                        functor: Name::with_intern(segment, self.gcx),
                        args: [empty_arg()].into(),
                    })
                    .collect();
                segments[n - 1].args = [parent_var].into();
                term::list_n(segments, self.gcx)
            }
        };

        // Function name term
        let fn_name_var = Term {
            functor: Name::with_intern(FN_NAME_VAR, self.gcx),
            args: [].into(),
        };
        let fn_name = Term {
            functor: Name::with_intern(fn_ident, self.gcx),
            args: [fn_name_var].into(),
        };

        // Signature term with argument variables. By replacing input arguments with variables
        // except the receiver, we can find all methods which require type coercions.
        let num_args = args.len();
        let var_args: Vec<TermIn<'gcx>> = (0..num_args)
            .map(|i| {
                // Receiver
                if i == 1 {
                    args[1].clone()
                } else {
                    Term {
                        functor: var_name(&i, self.gcx),
                        args: [].into(),
                    }
                }
            })
            .collect();
        let mut var_sig = term::sig_n(var_args, self.gcx);

        if !self.host.is_visible(parent_path, fn_ident)? {
            return err!(hard, "{parent_path}::{fn_ident} must be visible");
        }

        enum ReceiverState {
            T,       // T
            RefT,    // &T
            RefMutT, // &mut T
        }

        let mut recv_state = ReceiverState::T;

        loop {
            if self.resolve(parent.clone(), fn_name.clone(), var_sig.clone(), args) {
                break;
            }

            // We concern receiver(self) type only because it's a Rust rule. If other arguments are
            // not compatible with the function we found, then it is a compile error. In other
            // words, a method is determined by only the receiver, not by other parameters.
            // ref: https://doc.rust-lang.org/reference/expressions/method-call-expr.html

            match recv_state {
                // Automatical borrow: T -> ref(T)
                ReceiverState::T => {
                    let receiver = &mut var_sig.args[1];
                    *receiver = term::ref_1(receiver.clone(), self.gcx);
                    recv_state = ReceiverState::RefT;
                }
                // Automatical borrow: ref(T) -> ref(mut(T))
                ReceiverState::RefT => {
                    let receiver = &mut var_sig.args[1]; // ref(T)
                    let t = &mut receiver.args[0]; // T
                    *t = term::mut_1(t.clone(), self.gcx); // receiver = ref(mut(T))
                    recv_state = ReceiverState::RefMutT;
                }

                // Automatical dereference: Not yet implemented
                //
                // # Note
                // Dereferencing here means `Deref` trait. not dereferencing to references. Result
                // of dereferencing a reference is a value, not a type. See examples below.
                // e.g. `Add` trait is implemented for `i32`, but dereferencing to a `&mut i32`
                // cannot reach the implementation.
                // - `Add<i32> for &i32` is explicitly implemented.
                //   `let i: i32 = &0_i32 + 1_i32` // Ok
                // - `Add<i32> for &mut i32` doesn't exist.
                //   `let i: i32 = &mut 0_i32 + 1_i32` // Compile error
                ReceiverState::RefMutT => {
                    return err!(
                        hard,
                        "could not find a method for {parent_path}::{fn_ident}"
                    );
                }
            }
        }
        Ok(())
    }

    /// Tries to resolve the given arguments. If it succeeded, returns true.
    ///
    /// * var_sig - Signature term. e.g. sig($0, X, $1, $2)
    /// * args - Arguments that need to be resolved.
    ///
    /// # Note
    ///
    /// Every argument can have either only one varialble or zero. If not, it's undefined behavior.
    fn resolve(
        &mut self,
        parent: TermIn<'gcx>,
        fn_name: TermIn<'gcx>,
        var_sig: TermIn<'gcx>,
        args: &mut [TermIn<'gcx>],
    ) -> bool {
        // Looks for matching inherent method. If found, replaces the given arguments.
        let inher_fn = term::inher_fn_3(parent.clone(), fn_name.clone(), var_sig.clone(), self.gcx);
        if Self::_resolve(self.impl_logic, Expr::Term(inher_fn), args, self.gcx) {
            return true;
        }

        // Looks for matching trait method. If found, replaces the given arguments.
        let assoc_fn = term::assoc_fn_3(parent, fn_name, var_sig.clone(), self.gcx);
        Self::_resolve(self.impl_logic, Expr::Term(assoc_fn), args, self.gcx)
    }

    /// Tries to resolve the given arguments. If it succeeded, returns true.
    ///
    /// # Note
    ///
    /// Every argument can have either only one varialble or zero. If not, it's undefined behavior.
    fn _resolve(
        impl_logic: &mut ImplLogic<'gcx>,
        query: ExprIn<'gcx>,
        args: &mut [TermIn<'gcx>],
        gcx: &'gcx GlobalCx<'gcx>,
    ) -> bool {
        // Stores (nth of argument, corresponding term to the variable in the argument)
        let mut candidates = Vec::new();

        let mut cx = impl_logic.query(query);
        while let Some(eval) = cx.prove_next() {
            let old_num_candidates = candidates.len();

            for assign in eval {
                // Variables for function signature look like `$0`, `$1`, `$2`, and `$3` for
                // example. They each correspond to output, receiver, 1st input, and 2nd input.
                let Ok(nth) = assign.get_lhs_variable()[1..].parse::<usize>() else {
                    continue;
                };

                // If the given argument is not matching, then discards this assignment. But
                // receiver is an exception. It could be borrowed automatically.
                let mut rhs = assign.rhs();
                if nth != 1 && !Self::is_effective_same(&args[nth], &rhs) {
                    candidates.truncate(old_num_candidates);
                    break;
                }

                // We're going to replace variables only in the given arguments. So let's find out
                // corresponding term to the variable.
                if nth != 1 {
                    if let Some(dst) =
                        Self::find_corresponding_to_var_with_coercion(&args[nth], &rhs)
                    {
                        rhs = dst.clone();
                    }
                }

                let pair = (nth, rhs);
                if !candidates.contains(&pair) {
                    candidates.push(pair);
                }
            }
        }

        if candidates.is_empty() {
            return false;
        }

        while let Some((nth, mut rhs)) = candidates.pop() {
            let mut ambiguous_int = false;
            let mut ambiguous_float = false;

            for j in (0..candidates.len()).rev() {
                if candidates[j].0 == nth {
                    if Self::is_similar_int(&rhs, &candidates[j].1) {
                        ambiguous_int = true;
                    } else if Self::is_similar_float(&rhs, &candidates[j].1) {
                        ambiguous_float = true;
                    } else {
                        panic!("something went wrong: {}, {}", rhs, candidates[j].1);
                    }
                    candidates.swap_remove(j);
                }
            }

            assert!(!(ambiguous_int && ambiguous_float));

            if ambiguous_int {
                Self::replace_concrete_int(&mut rhs, nth, gcx);
            } else if ambiguous_float {
                Self::replace_concrete_float(&mut rhs, nth, gcx);
            }

            debug_assert!(Self::num_containing_vars(&args[nth]) <= 1);

            args[nth].replace_all(&mut |term| {
                if term.functor.starts_with(VAR_PREFIX)
                    && term.functor[1..].parse::<usize>() == Ok(nth)
                {
                    Some(rhs.clone())
                } else {
                    None
                }
            });
        }
        true
    }

    /// Finds a term in the `r` that corresponds to the first variable in the `l`. While searching,
    /// `l` can be coerced to other types for further searching.
    /// - e.g.
    ///   l: $X, r: a                -> no coercion     -> return Some(a)
    ///   l: ref(mut($X)), r: ref(a) -> ref($X), ref(a) -> return Some(a)
    ///
    /// # Note
    ///
    /// Undefined behavior if `r` contains variables.
    fn find_corresponding_to_var_with_coercion<'r>(
        l: &TermIn<'gcx>,
        r: &'r TermIn<'gcx>,
    ) -> Option<&'r TermIn<'gcx>> {
        if let Some(matching_term) = Self::find_var_corresponding(l, r) {
            return Some(matching_term);
        }

        // Tries various coercions.
        // ref: https://doc.rust-lang.org/reference/type-coercions.html

        if l.functor.as_ref() == term::FUNCTOR_REF {
            if l.args[0].functor.as_ref() == term::FUNCTOR_MUT {
                // &mut T -> &T
                let mut cloned = l.clone(); // ref(mut(..))
                let mut_ = &cloned.args[0]; // mut(..)
                cloned.args[0] = mut_.args[0].clone(); // ref(..) := ref(mut(..))
                Self::find_corresponding_to_var_with_coercion(&cloned, r)
            } else {
                // Not implemented yet
                None
            }
        } else {
            // Not implemented yet
            None
        }
    }

    /// Finds a term in the `r` that corresponds to the first variable in the `l`.
    /// - e.g. a($X, $Y), a(b, c) -> return Some(b)
    ///
    /// # Note
    ///
    /// Undefined behavior if `r` contains variables.
    fn find_var_corresponding<'r>(
        l: &TermIn<'gcx>,
        r: &'r TermIn<'gcx>,
    ) -> Option<&'r TermIn<'gcx>> {
        if !Self::is_effective_same(l, r) {
            None
        } else if l.is_variable() {
            Some(r)
        } else {
            l.args
                .iter()
                .zip(&r.args)
                .find_map(|(la, ra)| Self::find_var_corresponding(la, ra))
        }
    }

    /// If two terms are exactly the same, returns true. If `l` contains variables in it and `r` is
    /// the same with the `l` except vatiables, returns true.
    /// - e.g. `a(b($X), $Y)` and `a(b(c), d)` are the same
    ///
    /// # Note
    ///
    /// Undefined behavior if `r` contains variables.
    fn is_effective_same(l: &TermIn<'gcx>, r: &TermIn<'gcx>) -> bool {
        if l.is_variable() {
            return true;
        }
        l.functor == r.functor
            && l.args.len() == r.args.len()
            && l.args
                .iter()
                .zip(&r.args)
                .all(|(la, ra)| Self::is_effective_same(la, ra))
    }

    /// Returns true if the two given terms are the same except concrete integer type like below.
    ///
    /// - l: ...int(i8)...
    /// - r: ...int(i16)...
    fn is_similar_int(l: &TermIn<'gcx>, r: &TermIn<'gcx>) -> bool {
        if Self::CONCRETE_INTS.contains(&l.functor.as_ref())
            && Self::CONCRETE_INTS.contains(&r.functor.as_ref())
        {
            return true;
        }

        l.functor == r.functor
            && l.args.len() == r.args.len()
            && l.args
                .iter()
                .zip(&r.args)
                .any(|(la, ra)| Self::is_similar_int(la, ra))
    }

    /// Returns true if the two given terms are the same except concrete float type like below.
    ///
    /// - l: ...float(f32)...
    /// - r: ...float(f64)...
    fn is_similar_float(l: &TermIn<'gcx>, r: &TermIn<'gcx>) -> bool {
        if Self::CONCRETE_FLOATS.contains(&l.functor.as_ref())
            && Self::CONCRETE_FLOATS.contains(&r.functor.as_ref())
        {
            return true;
        }

        l.functor == r.functor
            && l.args.len() == r.args.len()
            && l.args
                .iter()
                .zip(&r.args)
                .any(|(la, ra)| Self::is_similar_float(la, ra))
    }

    /// Replaces concrete int type from the given term with a variable.
    ///
    /// e.g. ...int(i32)... -> ...int($0)...
    fn replace_concrete_int(term: &mut TermIn<'gcx>, var_ident: usize, gcx: &'gcx GlobalCx<'gcx>) {
        if Self::CONCRETE_INTS.contains(&term.functor.as_ref()) {
            term.functor = var_name(&var_ident, gcx);
        }

        for arg in &mut term.args {
            Self::replace_concrete_int(arg, var_ident, gcx);
        }
    }

    /// Replaces concrete int type from the given term with a variable.
    ///
    /// e.g. ...int(i32)... -> ...int($0)...
    fn replace_concrete_float(
        term: &mut TermIn<'gcx>,
        var_ident: usize,
        gcx: &'gcx GlobalCx<'gcx>,
    ) {
        if Self::CONCRETE_FLOATS.contains(&term.functor.as_ref()) {
            term.functor = var_name(&var_ident, gcx);
        }

        for arg in &mut term.args {
            Self::replace_concrete_float(arg, var_ident, gcx);
        }
    }

    /// Returns number of variables in the given term.
    fn num_containing_vars(term: &TermIn<'gcx>) -> usize {
        if term.is_variable() {
            return 1;
        }
        term.args
            .iter()
            .map(Self::num_containing_vars)
            .sum::<usize>()
    }
}
