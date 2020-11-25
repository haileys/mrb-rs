use std::ffi::CString;
use std::mem;
use std::os::raw::c_void;
use std::panic;
use std::process;
use std::ptr;

use crate::{MrbPtr, MrbResult, Context};
use crate::object::{MrbValue, MrbClass, MrbException};

type BoxedFunc = Box<dyn for<'sub> Fn(&Context<'sub>, MrbValue<'sub>) -> MrbResult<'sub, MrbValue<'sub>> + 'static>;

#[no_mangle]
unsafe extern "C" fn mrbrs_method_free_boxed_func(_mrb: *mut mrb_sys::mrb_state, ptr: *mut c_void) {
    let result = panic::catch_unwind(|| {
        mem::drop(Box::from_raw(ptr as *mut BoxedFunc));
    });

    match result {
        Ok(()) => {}
        Err(e) => {
            // TODO stash this panic somewhere so we can resume unwind once
            // we're back in Rust on the other side
            eprintln!("PANIC while dropping BoxedFunc in mrbrs_method_free_boxed_func: {:?}", e);
            eprintln!("Cannot unwind, aborting");
            process::abort();
        }
    }
}

#[no_mangle]
unsafe extern "C" fn mrbrs_method_dispatch_boxed_func(
    mrb: *mut mrb_sys::mrb_state,
    value: mrb_sys::mrb_value,
    data: *mut c_void,
) -> mrb_sys::mrb_value {
    let ctx = Context::new(mrb);
    let func = data as *mut BoxedFunc;

    let result = panic::catch_unwind(|| {
        (*func)(&ctx, MrbValue::new(value))
    });

    match result {
        // normal return:
        Ok(Ok(val)) => val.as_raw(),

        // ruby exception:
        Ok(Err(ex)) => {
            // TODO make this a proper ruby exception
            eprintln!("exception from Rust method! {:?}", e);
            process::abort();
        }

        // rust panic:
        Err(panic) => {
            // TODO stash this panic somewhere so we can resume unwind once
            // we're back in Rust on the other side
            eprintln!("PANIC while calling Rust method in mrbrs_method_dispatch_boxed_func: {:?}", e);
            eprintln!("Cannot unwind, aborting");
            process::abort();
        }
    }
}

impl<'mrb> Context<'mrb> {
    pub fn define_method<F>(&self, class: MrbClass<'mrb>, name: &str, func: F) -> MrbResult<'mrb, ()>
        where F: for<'sub> Fn(&Context<'sub>, MrbValue<'sub>) -> MrbResult<'sub, MrbValue<'sub>> + 'static
    {
        let name = CString::new(name).expect("CString::from");

        // we need to double box here because trait object boxes are fat pointers
        let func = Box::into_raw(Box::new(Box::new(func) as BoxedFunc));

        let mut exc = ptr::null_mut();

        let proc_ = self.boundary(|| unsafe {
            mrb_sys::mrbrs_method_make_boxed_func(
                self.mrb,
                func as *mut c_void,
                &mut exc as *mut _,
            )
        })?;

        if proc_ == ptr::null_mut() {
            return Err(unsafe { MrbException(MrbPtr::new(self.mrb, exc)) });
        }

        self.boundary(|| unsafe {
            mrb_sys::mrbrs_define_method_proc(
                self.mrb,
                class.0.as_ptr(),
                name.as_ptr(),
                proc_,
                &mut exc as *mut _,
            );
        })?;

        if exc != ptr::null_mut() {
            return Err(unsafe { MrbException(MrbPtr::new(self.mrb, exc)) });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Mrb;

    #[test]
    fn test_define_method() {
        let mut mrb = Mrb::open();

        mrb.try_context(|mrb| {
            mrb.define_method(mrb.object_class(), "my_method", |_ctx, _self| {
                panic!("rust method!")
            })?;

            mrb.load_string("my_method")?;

            Ok(())
        }).expect("try_context");
    }
}
