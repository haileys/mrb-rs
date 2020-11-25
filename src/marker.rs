use std::cell::Cell;
use std::marker::PhantomData;
use std::panic::{UnwindSafe, RefUnwindSafe};

#[derive(Copy, Clone)]
pub struct Invariant<'a>(PhantomData<Cell<&'a ()>>);

impl<'a> Invariant<'a> {
    pub fn phantom() -> Self {
        Invariant(PhantomData)
    }
}

// Invariant is a zero-sized marker type, so we can conveniently mark it as
// unwind safe to prevent us from having to use AssertUnwindSafe elsewhere
impl<'a> UnwindSafe for Invariant<'a> {}
impl<'a> RefUnwindSafe for Invariant<'a> {}
