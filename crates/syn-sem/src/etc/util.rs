use crate::{Map, Result};
use std::{
    borrow::Cow,
    cell::RefCell,
    ffi::OsStr,
    fmt::{self, Write},
    hash::Hash,
    hash::Hasher,
    str,
    sync::{Mutex, MutexGuard},
};

pub(crate) fn os_str_to_str(os: &OsStr) -> Result<&str> {
    if let Some(s) = os.to_str() {
        Ok(s)
    } else {
        Err(format!("`{os:?}` is not a UTF-8 literal").into())
    }
}

pub(crate) fn push_colon_path<S>(dst: &mut String, add: S)
where
    S: fmt::Display,
{
    if dst.is_empty() || dst.ends_with("::") {
        write!(dst, "{}", add)
    } else {
        write!(dst, "::{}", add)
    }
    .unwrap();
}

pub(crate) struct FiniteLoop;

impl FiniteLoop {
    thread_local! {
        static COUNTERS: RefCell<Map<&'static str, Map<u64, u32>>> = RefCell::new(
            Map::default()
        );
        static LIMITS: RefCell<Map<&'static str, u32>> = RefCell::new(
            Map::default()
        );
    }

    pub(crate) fn set_limit(id: &'static str, limit: u32) {
        Self::LIMITS.with(|limits| {
            let mut limits = limits.borrow_mut();
            limits.entry(id).or_insert(limit);
        });
    }

    pub(crate) fn reset(id: &str) {
        Self::COUNTERS.with(|counters| {
            let mut counters = counters.borrow_mut();
            if let Some(counter) = counters.get_mut(id) {
                counter.clear();
            }
        });
    }

    pub(crate) fn assert<I, II, F>(id: &'static str, keys: I, on_error: F)
    where
        I: Iterator<Item = II>,
        II: Hash,
        F: FnOnce(),
    {
        let limit = Self::LIMITS.with(|limits| {
            let limits = limits.borrow();
            limits.get(id).cloned().unwrap_or(10)
        });

        let mut hasher = fxhash::FxHasher::default();
        for key in keys {
            key.hash(&mut hasher);
        }
        let hash = hasher.finish();

        Self::COUNTERS.with(|counters| {
            let mut counters = counters.borrow_mut();
            let counter = counters.entry(id).or_default();
            counter
                .entry(hash)
                .and_modify(|cnt| {
                    *cnt -= 1;
                    if *cnt == 0 {
                        on_error();
                    }
                })
                .or_insert(limit);
        })
    }
}

pub trait IntoPathSegments: Clone {
    type Item: AsRef<str>;
    type Iter: Iterator<Item = Self::Item> + Clone;

    fn segments(self) -> Self::Iter;
}

impl<'a> IntoPathSegments for &'a str {
    type Item = &'a str;
    type Iter = Filter<str::Split<'a, &'a str>>;

    fn segments(self) -> Self::Iter {
        Filter {
            segments: self.split("::"),
        }
    }
}

#[derive(Clone)]
pub struct Filter<I> {
    segments: I,
}

impl<'a, I: Iterator<Item = &'a str>> Iterator for Filter<I> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.segments
            .by_ref()
            .find(|&segment| !segment.is_empty())
            .map(|v| v as _)
    }
}

#[derive(Debug, Clone)]
pub struct PathSegments<I>(pub I);

impl<I, II> IntoPathSegments for PathSegments<I>
where
    I: Iterator<Item = II> + Clone,
    II: AsRef<str>,
{
    type Item = II;
    type Iter = I;

    fn segments(self) -> Self::Iter {
        self.0
    }
}

static CRATE_NAME: Mutex<Cow<'static, str>> = Mutex::new(Cow::Borrowed("crate"));

/// Sets crate name without validation.
///
/// * Must not be empty
/// * Must start with a letter
/// * Must consist of lowercase letters(a-z), numbers(0-9), or underscores(_)
/// * And mores
pub fn set_crate_name<T: Into<Cow<'static, str>>>(name: T) {
    *CRATE_NAME.lock().unwrap() = name.into();
}

pub fn get_crate_name() -> MutexGuard<'static, Cow<'static, str>> {
    CRATE_NAME.lock().unwrap()
}

pub fn cargo_crate_name() -> Cow<'static, str> {
    let pkg_name = env!("CARGO_PKG_NAME");
    if !pkg_name.contains('-') {
        Cow::Borrowed(pkg_name)
    } else {
        Cow::Owned(pkg_name.replace('-', "_"))
    }
}

/// Creates string which looks like "a::b::C" from the given path ignoring
/// leading colon.
pub(crate) fn get_name_path_from_syn_path(path: &syn::Path) -> String {
    let mut buf = String::new();
    for segment in &path.segments {
        push_colon_path(&mut buf, &segment.ident);
    }
    buf
}
