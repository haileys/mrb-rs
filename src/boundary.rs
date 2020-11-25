use std::any::Any;
use std::mem;
use std::os::raw::c_void;
use std::panic::{self, UnwindSafe};
use std::process;
use std::ptr::{self, NonNull};

use mrb_sys::mrb_state;

use crate::MrbResult;
use crate::object::{MrbPtr, MrbException};

type PanicSlot = Option<Box<dyn Any + Send>>;

/// Safety: takes raw pointer and ascribes lifetime to result. Care must be taken.
pub unsafe fn into_mruby<'mrb, R>(mrb: *mut mrb_state, f: impl FnOnce() -> R) -> MrbResult<'mrb, R> {
    let mut panic_info: PanicSlot = None;

    // install pointer to panic_info in mrb_state's user data field
    let mut prev_ud = &mut panic_info as *mut PanicSlot as *mut c_void;
    mem::swap(&mut prev_ud, &mut (*mrb).ud);

    // call into mruby
    let result = f();

    // restore previous panic_info pointer if exists
    mem::swap(&mut prev_ud, &mut (*mrb).ud);

    // check for panic and resume unwind if necessary
    if let Some(panic_info) = panic_info {
        panic::resume_unwind(panic_info);
    }

    // check for ruby exception and translate to rust error
    let mut exc = ptr::null_mut();
    mem::swap(&mut exc, &mut (*mrb).exc);

    if exc == ptr::null_mut() {
        Ok(result)
    } else {
        Err(MrbException(MrbPtr::new(mrb, exc)))
    }
}

/// Safety takes raw pointer
pub unsafe fn into_rust<R>(mrb: *mut mrb_state, f: impl FnOnce() -> R + UnwindSafe) -> Result<R, ()> {
    let mut panic_slot = match NonNull::new((*mrb).ud as *mut PanicSlot) {
        Some(slot) => slot,
        None => {
            eprintln!("*** No Rust panic handler installed in MRuby context! Cannot unwind, aborting");
            process::abort();
        }
    };

    panic::catch_unwind(f).map_err(|panic| {
        *panic_slot.as_mut() = Some(panic);
    })
}
