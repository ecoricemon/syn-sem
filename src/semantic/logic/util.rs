use super::term;
use crate::{semantic::entry::GlobalCx, NameIn, TermIn};
use logic_eval::{Name, Term, VAR_PREFIX};
use std::{fmt, num::ParseIntError, result::Result as StdResult};

pub(crate) fn var_name<'gcx, T>(t: &T, gcx: &'gcx GlobalCx<'gcx>) -> NameIn<'gcx>
where
    T: fmt::Display + ?Sized,
{
    let name = format!("{VAR_PREFIX}{}", t);
    Name::with_intern(&name, gcx)
}

// If the functor is an integer or floating types, wrap it in 'int()' or 'float()'. This makes us
// to be able to represent an ambigous types such as 'int($X)' or 'float($X)'.
pub(crate) fn try_make_int_or_float_term<'gcx>(
    functor: &str,
    gcx: &'gcx GlobalCx<'gcx>,
) -> Option<TermIn<'gcx>> {
    match functor {
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128"
        | "usize" => {
            let int = Term {
                functor: Name::with_intern(functor, gcx),
                args: [].into(),
            };
            Some(term::int_1(int, gcx))
        }
        "f32" | "f64" => {
            let float = Term {
                functor: Name::with_intern(functor, gcx),
                args: [].into(),
            };
            Some(term::float_1(float, gcx))
        }
        _ => None,
    }
}

pub(crate) fn ptr_to_name<'gcx, T>(t: *const T, gcx: &'gcx GlobalCx<'gcx>) -> NameIn<'gcx> {
    let ptr = t as *const () as usize;
    let name = format!("{ptr:x}");
    Name::with_intern(&name, gcx)
}

pub(crate) fn name_to_ptr<T>(name: &str) -> StdResult<*const T, ParseIntError> {
    let ptr = usize::from_str_radix(name, 16)?;
    Ok(ptr as *const () as *const T)
}
