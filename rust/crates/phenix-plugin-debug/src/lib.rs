#![forbid(unsafe_code)]

#[phenix_sdk::plugin]
pub struct Plugin;

mod component;
mod implementation;
pub use component::*;
pub use implementation::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_struct_owns_static_identity() {
        assert_eq!(Plugin::plugin_id().as_str(), "phenix.debug");
        assert_eq!(Plugin::component_id().as_str(), "phenix.debug");
    }
}
