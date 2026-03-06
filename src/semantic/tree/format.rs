use super::PathId;
use crate::{etc::abs_fs::AbstractFiles, Map};
use std::fmt::{self, Write};

pub struct Brief<'a, T: DebugBriefly> {
    data: &'a T,
    filter: PrintFilter,
}

impl<'a, T: DebugBriefly> Brief<'a, T> {
    /// Wraps the data within [`Brief`] type to make it to be printed concisely.
    pub fn new(data: &'a T) -> Self {
        Self {
            data,
            filter: PrintFilter {
                is_item_of: Map::default(),
                starts_with: Map::default(),
                contains: Map::default(),
            },
        }
    }

    /// Hides known libraries.
    pub fn hide_known(&mut self, files: &AbstractFiles) -> &mut Self {
        let known_names = files.known_libraries().map(|(name, _)| &**name);
        self.starts_with_then_hide(known_names)
    }

    pub fn starts_with_then_show_detail<'i, I>(&mut self, starts_with: I) -> &mut Self
    where
        I: IntoIterator<Item = &'i str>,
    {
        for s in starts_with {
            self.filter
                .starts_with
                .insert(s.to_owned(), DebugBriefHow::ShowDetail);
        }
        self
    }

    pub fn starts_with_then_hide<'i, I>(&mut self, starts_with: I) -> &mut Self
    where
        I: IntoIterator<Item = &'i str>,
    {
        for s in starts_with {
            self.filter
                .starts_with
                .insert(s.to_owned(), DebugBriefHow::Hide);
        }
        self
    }

    pub fn contains_then_show_detail<'i, I>(&mut self, contains: I) -> &mut Self
    where
        I: IntoIterator<Item = &'i str>,
    {
        for s in contains {
            self.filter
                .contains
                .insert(s.to_owned(), DebugBriefHow::ShowDetail);
        }
        self
    }

    pub fn contains_then_hide<'i, I>(&mut self, contains: I) -> &mut Self
    where
        I: IntoIterator<Item = &'i str>,
    {
        for s in contains {
            self.filter
                .contains
                .insert(s.to_owned(), DebugBriefHow::Hide);
        }
        self
    }

    pub fn is_item_of_then_show_detail<'i, I>(&mut self, item_names: I) -> &mut Self
    where
        I: IntoIterator<Item = &'i str>,
    {
        for s in item_names {
            self.filter
                .is_item_of
                .insert(s.to_owned(), DebugBriefHow::ShowDetail);
        }
        self
    }

    pub fn is_item_of_then_hide<'i, I>(&mut self, item_names: I) -> &mut Self
    where
        I: IntoIterator<Item = &'i str>,
    {
        for s in item_names {
            self.filter
                .is_item_of
                .insert(s.to_owned(), DebugBriefHow::Hide);
        }
        self
    }
}

impl<T: DebugBriefly> fmt::Debug for Brief<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.fmt_briefly(f, &self.filter)
    }
}

pub(crate) struct DebugItem<'a, T> {
    pub(crate) id: &'a PathId,
    pub(crate) path: &'a String,
    pub(crate) item: &'a T,
}

impl<T: fmt::Debug> fmt::Debug for DebugItem<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.id, f)?;
        f.write_char(' ')?;
        fmt::Debug::fmt(&self.path, f)?;
        f.write_str(" => ")?;
        self.item.fmt(f)
    }
}

pub(crate) struct BriefDebugItem<'a, T> {
    pub(crate) id: &'a PathId,
    pub(crate) path: &'a String,
    pub(crate) item: &'a T,
    pub(crate) filter: &'a PrintFilter,
}

impl<T: DebugBriefly> fmt::Debug for BriefDebugItem<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.id, f)?;
        f.write_char(' ')?;
        fmt::Debug::fmt(&self.path, f)?;
        f.write_str(" => ")?;
        self.item.fmt_briefly(f, self.filter)
    }
}

pub trait DebugBriefly: fmt::Debug {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, filter: &PrintFilter) -> fmt::Result;

    fn name(&self) -> &'static str;
}

// Priority of filtering is determined by each implementation, but it is recommended to follow
// field declaration order.
pub struct PrintFilter {
    pub(crate) is_item_of: Map<String, DebugBriefHow>,
    pub(crate) starts_with: Map<String, DebugBriefHow>,
    pub(crate) contains: Map<String, DebugBriefHow>,
}

#[derive(Clone, Copy)]
pub enum DebugBriefHow {
    ShowDetail,
    Hide,
}
