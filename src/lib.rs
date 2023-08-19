#![no_std]

extern crate alloc;

pub mod animation;
pub mod color;
mod color_cache;
pub mod controller;
pub mod curve;
pub mod indexing;
pub mod pattern;
pub mod processing;
pub mod strip;
pub mod timeline;
mod util;

#[cfg(test)]
mod mock;
