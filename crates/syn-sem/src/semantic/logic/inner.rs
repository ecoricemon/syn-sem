use super::{
    construct,
    find_method::{self, MethodFinder},
    ImplLogic,
};
use crate::{semantic::entry::GlobalCx, TermIn, TriResult};

#[derive(Debug)]
pub struct Logic<'gcx> {
    gcx: &'gcx GlobalCx<'gcx>,
    pub impl_: ImplLogic<'gcx>,
}

impl<'gcx> Logic<'gcx> {
    pub(crate) fn new(gcx: &'gcx GlobalCx<'gcx>) -> Self {
        Self {
            gcx,
            impl_: ImplLogic::new(gcx),
        }
    }

    pub(crate) fn load_file<H: construct::Host<'gcx>>(
        &mut self,
        host: &mut H,
        file: &syn::File,
    ) -> TriResult<(), ()> {
        self.impl_.load_file(host, file)
    }

    pub(crate) fn load_item_impl<H: construct::Host<'gcx>>(
        &mut self,
        host: &mut H,
        item_impl: &syn::ItemImpl,
    ) -> TriResult<(), ()> {
        self.impl_.load_item_impl(item_impl, host)
    }

    pub(crate) fn find_method_sig<H: find_method::Host<'gcx>>(
        &mut self,
        host: &mut H,
        parent_path: &str,
        fn_ident: &str,
        args: &mut [TermIn<'gcx>],
    ) -> TriResult<(), ()> {
        MethodFinder::new(self.gcx, &mut self.impl_, host).find_method_sig(
            parent_path,
            fn_ident,
            args,
        )
    }
}
