pub use mrb_sys as sys;

use std::borrow::Cow;
use std::convert::TryInto;
use std::ffi::CString;
use std::os::raw::c_int;
use std::ptr;
use std::slice;

mod marker;
mod method;
mod object;
mod state;

pub use object::{MrbValue, MrbObject, MrbClass, MrbException};

use object::MrbPtr;
use marker::Invariant;
use state::MrbState;

pub struct Mrb {
    state: MrbState,
}

impl Mrb {
    pub fn open() -> Self {
        let state = MrbState::open().expect("MrbState::open");
        Mrb { state }
    }

    pub fn context<Ret>(&mut self, f: impl for<'mrb> FnOnce(&Context<'mrb>) -> Ret) -> Ret {
        let ctx = unsafe { Context::new(self.state.as_ptr()) };
        f(&ctx)
    }

    pub fn try_context<Ret>(&mut self, f: impl for<'mrb> FnOnce(&Context<'mrb>) -> MrbResult<'mrb, Ret>) -> Result<Ret, String> {
        let ctx = unsafe { Context::new(self.state.as_ptr()) };
        f(&ctx).map_err(|e| format!("{:?}", e))
    }
}

pub struct Context<'mrb> {
    mrb: *mut sys::mrb_state,

    // all objects returned to Rust while working with a context are saved in
    // the arena so they don't get garbage collected from underneath us. on
    // drop we roll back the arena to its previous index
    arena_index: c_int,

    // this invariant marker lets us brand references to mruby objects with
    // the particular state instance:
    _invariant: Invariant<'mrb>,
}

impl<'mrb> Drop for Context<'mrb> {
    fn drop(&mut self) {
        unsafe { sys::mrbrs_gc_arena_restore(self.mrb, self.arena_index) };
    }
}

pub type MrbResult<'mrb, T> = Result<T, MrbException<'mrb>>;

impl<'mrb> Context<'mrb> {
    unsafe fn new(mrb: *mut sys::mrb_state) -> Self {
        let arena_index = sys::mrbrs_gc_arena_save(mrb);

        Context {
            mrb,
            arena_index,
            _invariant: Invariant::phantom(),
        }
    }

    pub fn object_class(&self) -> MrbClass<'mrb> {
        MrbClass(unsafe {
            MrbPtr::new(self.mrb, (*self.mrb).object_class)
        })
    }

    pub fn define_class(&self, name: &str, superclass: MrbClass<'mrb>) -> MrbResult<'mrb, MrbClass<'mrb>> {
        let name = CString::new(name).expect("CString::from");

        let mut exc = ptr::null_mut();

        let ptr = unsafe {
            sys::mrbrs_define_class(
                self.mrb,
                name.as_ptr(),
                superclass.0.as_ptr(),
                &mut exc as *mut *mut _,
            )
        };

        if ptr == ptr::null_mut() {
            Err(unsafe { MrbException(MrbPtr::new(self.mrb, exc)) })
        } else {
            Ok(unsafe { MrbClass(MrbPtr::new(self.mrb, ptr)) })
        }
    }

    pub fn arguments(&self) -> &'mrb [MrbValue<'mrb>] {
        unsafe {
            let argc = sys::mrb_get_argc(self.mrb);
            let argv = sys::mrb_get_argv(self.mrb);
            let ptr = argv as *const _ as *const MrbValue<'mrb>;
            slice::from_raw_parts(ptr, argc.try_into().unwrap())
        }
    }

    pub fn inspect(&self, value: MrbValue<'mrb>) -> Cow<'mrb, str> {
        unsafe { object::inspect(self.mrb, value) }
    }

    pub fn load_string(&self, code: &str) -> MrbResult<'mrb, MrbValue<'mrb>> {
        let mut exc = ptr::null_mut();

        let result = unsafe {
            sys::mrbrs_load_nstring(
                self.mrb,
                code.as_ptr() as *const i8,
                code.len().try_into().unwrap(),
                &mut exc,
            )
        };

        if exc == ptr::null_mut() {
            Ok(unsafe { MrbValue::new(result) })
        } else {
            Err(unsafe { MrbException(MrbPtr::new(self.mrb, exc)) })
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Mrb;

    #[test]
    fn test_open_close() {
        Mrb::open();
    }

    #[test]
    fn test_define_class() {
        let mut mrb = Mrb::open();

        mrb.context(|mrb| {
            let cls1 = mrb.define_class("MyClass", mrb.object_class())
                .expect("first define_class");

            let cls2 = mrb.define_class("MyClass", cls1);
            let err = cls2.unwrap_err();
            let msg = format!("{:?}", err);
            assert_eq!(msg, "superclass mismatch for Class MyClass (Object not MyClass) (TypeError)");
        });
    }

    #[test]
    fn test_arguments() {
        let mut mrb = Mrb::open();

        mrb.context(|mrb| {
            assert_eq!(0, mrb.arguments().len());
        });
    }

    #[test]
    fn test_load_string() {
        let mut mrb = Mrb::open();

        mrb.context(|mrb| {
            let eval = |code: &str| -> Result<String, String> {
                mrb.load_string(code)
                    .map(|val| mrb.inspect(val).to_string())
                    .map_err(|err| format!("{:?}", err))
            };

            assert_eq!("3", eval("1 + 2").unwrap());
            assert_eq!("hello (RuntimeError)", eval("raise 'hello'").unwrap_err());
            assert_eq!("syntax error (SyntaxError)", eval("$%^&#$W").unwrap_err());
        });
    }
}
