use std::{
    any::{Any, TypeId},
    fmt, hash,
};

pub trait IdentifySyn: Any {
    fn as_any(&self) -> &dyn Any;

    fn syn_id(&self) -> SynId
    where
        Self: Sized,
    {
        SynId {
            trait_ptr: self as *const Self as *const dyn IdentifySyn,
            type_id: self.type_id(),
        }
    }

    fn content(&self) -> String {
        self.type_name().to_owned()
    }

    fn type_name(&self) -> &'static str;
}

#[derive(Clone, Copy)]
pub struct SynId {
    trait_ptr: *const dyn IdentifySyn,

    /// Supports unique syn node identification.
    ///
    /// # Why trait pointer is not sufficient
    ///
    /// * Metadata(vtable pointer) of the trait pointer cannot be used for identification.
    ///   - See https://doc.rust-lang.org/std/ptr/struct.DynMetadata.html
    /// * Data address of the trait pointer is not sufficient.
    ///   - A transparent type would have the same data address as what its child has.
    /// * As a result, we need more data for the unique syn node identification.
    type_id: TypeId,
}

impl SynId {
    pub fn content(&self) -> String {
        unsafe { self.trait_ptr.as_ref().unwrap() }.content()
    }

    pub fn as_identify_syn(&self) -> &dyn IdentifySyn {
        unsafe { self.trait_ptr.as_ref().unwrap() }
    }

    pub fn as_any(&self) -> &dyn Any {
        let r = unsafe { self.trait_ptr.as_ref().unwrap() };
        r.as_any()
    }

    pub fn as_ref<T: Any>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    pub fn as_const_ptr<T: Any>(&self) -> Option<*const T> {
        self.as_ref().map(|ref_| ref_ as *const T)
    }

    pub fn type_name(&self) -> &'static str {
        unsafe { self.trait_ptr.as_ref().unwrap() }.type_name()
    }
}

impl PartialEq for SynId {
    fn eq(&self, other: &Self) -> bool {
        // Ignores metadata (vtable pointer)
        self.trait_ptr as *const () == other.trait_ptr as *const () && self.type_id == other.type_id
    }
}

impl Eq for SynId {}

impl hash::Hash for SynId {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        // Ignores metadata (vtable pointer)
        (self.trait_ptr as *const ()).hash(state);
        self.type_id.hash(state);
    }
}

impl fmt::Debug for SynId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.trait_ptr.fmt(f)
    }
}

impl fmt::Display for SynId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
