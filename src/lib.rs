#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod animation;
pub mod color;
mod color_cache;
mod curve;
pub mod indexing;
pub mod strip;
pub mod timeline;
mod util;

#[cfg(test)]
mod mock;
