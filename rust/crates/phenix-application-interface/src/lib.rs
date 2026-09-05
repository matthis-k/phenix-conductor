#![forbid(unsafe_code)]

//! Fixed application contracts. This crate describes behavior without owning runtime state.
//! Protocol adapters and generators consume the same descriptor and structural vocabulary.

#[cfg(test)]
extern crate self as phenix_application_interface;

mod catalog;
mod client;
mod descriptor;
pub mod generate;
pub mod types;

pub use catalog::*;
pub use client::*;
pub use descriptor::*;

pub const INTERFACE_ID: &str = "phenix.application@1";
pub const SNAPSHOT: &str = "share/phenix/interfaces/phenix.application@1.json";

#[cfg(test)]
mod tests;
