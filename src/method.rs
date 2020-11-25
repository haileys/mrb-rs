use std::ffi::CString;
use std::mem;
use std::os::raw::c_void;
use std::panic;
use std::process;

use crate::{MrbResult, Context};
use crate::boundary;
use crate::object::{MrbValue, MrbClass};

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
    retn: &mut mrb_sys::mrb_value,
) {
    let ctx = Context::new(mrb);

    let result = boundary::into_rust(mrb, || {
        let func = data as *mut BoxedFunc;
        (*func)(&ctx, MrbValue::new(value))
    });

    match result {
        // normal return:
        Ok(Ok(val)) => {
            *retn = val.as_raw();
        }

        // ruby exception:
        Ok(Err(ex)) => {
            (*mrb).exc = ex.0.as_ptr();
        }

        // rust panic:
        Err(()) => {
            let ud = (*mrb).ud as *const mrb_sys::mrbrs_ud;
            let carrier = (*ud).panic_carrier;
            (*mrb).exc = carrier;
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

        let proc_ = self.boundary(|| unsafe {
            mrb_sys::mrbrs_method_make_boxed_func(
                self.mrb,
                func as *mut c_void,
            )
        })?;

        self.boundary(|| unsafe {
            mrb_sys::mrbrs_define_method_proc(
                self.mrb,
                class.0.as_ptr(),
                name.as_ptr(),
                proc_,
            );
        })?;

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
            mrb.define_method(mrb.object_class(), "my_method", |_ctx, self_| {
                // just return self
                Ok(self_)
            })?;

            mrb.load_string("my_method")?;

            Ok(())
        }).expect("try_context");
    }

    #[test]
    fn test_panicking_method() {
        use std::panic;

        let mut mrb = Mrb::open();

        mrb.try_context(|mrb| {
            mrb.define_method(mrb.object_class(), "my_method", |_ctx, _self| {
                // just return self
                panic!("this is a rust panic!")
            })?;

            let result = panic::catch_unwind(|| {
                let _ = mrb.load_string(r#"
                    begin
                        my_method
                    rescue BasicObject => e
                        # test that we can't catch rust panics
                    end
                "#);
            });

            assert!(result.is_err());

            Ok(())
        }).expect("try_context");
    }

    #[test]
    fn test_raising_method() {
        let mut mrb = Mrb::open();

        mrb.try_context(|mrb| {
            mrb.define_method(mrb.object_class(), "my_method", |ctx, _self| {
                // we have to use load_string to create an exception instance for now...
                ctx.load_string("raise 'hello'")
            })?;

            let result = mrb.load_string("my_method");
            assert_eq!("Err(hello (RuntimeError))", format!("{:?}", result));

            Ok(())
        }).expect("try_context");
    }
}
