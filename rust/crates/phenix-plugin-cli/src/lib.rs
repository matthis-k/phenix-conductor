#![forbid(unsafe_code)]
mod component;
mod implementation {
    include!("implementation.rs");
}
pub use component::*;
pub use implementation::*;
