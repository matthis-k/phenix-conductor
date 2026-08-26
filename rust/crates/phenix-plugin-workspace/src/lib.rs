#![forbid(unsafe_code)]
mod implementation {
    include!("implementation.rs");
}
pub use implementation::*;
