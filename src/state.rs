use std::ptr;
use mrb_sys as sys;

pub(crate) struct MrbState(*mut sys::mrb_state);

impl MrbState {
    pub fn open() -> Result<Self, ()> {
        let state = unsafe {
            sys::mrbrs_open_core()
        };

        if state == ptr::null_mut() {
            Err(())
        } else {
            Ok(MrbState(state))
        }
    }

    pub fn as_ptr(&self) -> *mut sys::mrb_state {
        self.0
    }
}

impl Drop for MrbState {
    fn drop(&mut self) {
        unsafe {
            sys::mrbrs_close(self.0);
        }
    }
}
