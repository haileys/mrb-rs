use std::cell::Cell;
use std::marker::PhantomData;

pub type Invariant<'a> = PhantomData<Cell<&'a ()>>;
