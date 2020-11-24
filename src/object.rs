use std::borrow::Cow;
use std::convert::TryInto;
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::slice;

use crate::marker::Invariant;
use crate::state::MrbState;

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct MrbValue<'mrb> {
    value: mrb_sys::mrb_value,
    _inv: Invariant<'mrb>,
}

impl<'mrb> MrbValue<'mrb> {
    pub(crate) unsafe fn new(value: mrb_sys::mrb_value) -> Self {
        MrbValue {
            value,
            _inv: PhantomData,
        }
    }
}

pub struct MrbPtr<'mrb, T> {
    state: &'mrb MrbState,
    ptr: *mut T,
}

impl<'mrb, T> MrbPtr<'mrb, T> {
    /// Safety: you guarantee that `ptr` has lifetime `'mrb`
    pub(crate) unsafe fn new(state: &'mrb MrbState, ptr: *mut T) -> Self {
        MrbPtr { state, ptr }
    }

    pub(crate) fn as_ptr(&self) -> *mut T {
        self.ptr
    }

    pub(crate) unsafe fn cast<U>(self) -> MrbPtr<'mrb, U> {
        MrbPtr {
            state: self.state,
            ptr: self.ptr as *mut U,
        }
    }

    pub(crate) fn inspect(&self) -> Cow<'mrb, str> {
        unsafe {
            let value = mrb_sys::mrbrs_obj_value(self.ptr as *mut _);
            inspect(self.state, MrbValue::new(value))
        }
    }
}

impl<'mrb, T> Debug for MrbPtr<'mrb, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.ptr)
    }
}

#[derive(Debug)]
pub struct MrbObject<'mrb>(pub(crate) MrbPtr<'mrb, mrb_sys::RObject>);

impl<'mrb> MrbObject<'mrb> {
    pub fn inspect(&self) -> Cow<'mrb, str> {
        self.0.inspect()
    }
}

#[derive(Debug)]
pub struct MrbClass<'mrb>(pub(crate) MrbPtr<'mrb, mrb_sys::RClass>);

impl<'mrb> Into<MrbObject<'mrb>> for MrbClass<'mrb> {
    fn into(self) -> MrbObject<'mrb> {
        MrbObject(unsafe { self.0.cast() })
    }
}

pub struct MrbException<'mrb>(pub(crate) MrbPtr<'mrb, mrb_sys::RObject>);

impl<'mrb> Into<MrbObject<'mrb>> for MrbException<'mrb> {
    fn into(self) -> MrbObject<'mrb> {
        MrbObject(unsafe { self.0.cast() })
    }
}

impl<'mrb> Debug for MrbException<'mrb> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.inspect())
    }
}

pub(crate) fn inspect<'mrb>(state: &'mrb MrbState, value: MrbValue<'mrb>) -> Cow<'mrb, str> {
    unsafe {
        let mut len: mrb_sys::size_t = 0;
        let ptr = mrb_sys::mrbrs_inspect(state.as_ptr(), value.value, &mut len as *mut _);
        let bytes = slice::from_raw_parts(ptr as *const u8, len.try_into().unwrap());

        // Safety: mrbrs_inspect freezes and GC protects the string so we
        // know the underlying buffer will be valid for the 'mrb lifetime
        String::from_utf8_lossy(bytes)
    }
}
