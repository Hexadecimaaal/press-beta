#![cfg_attr(not(feature = "std"), no_std)]
#![feature(maybe_uninit_uninit_array)]
#![feature(maybe_uninit_array_assume_init)]

extern crate alloc;

#[cfg(feature = "std")]
pub mod test;

pub mod heap;
pub mod lambda;
