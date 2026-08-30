use crate::SESSION_TREE_SERVICE;
use phenix_core::{ComponentInterface, InterfaceId};

pub struct SessionTreeInterface;

impl ComponentInterface for SessionTreeInterface {
    fn interface_id() -> InterfaceId {
        InterfaceId::parse(SESSION_TREE_SERVICE).expect("static session-tree interface id is valid")
    }

    fn schema() -> phenix_core::InterfaceSchema {
        phenix_core::InterfaceSchema::of::<crate::SessionTreeCommand, crate::SessionTreeResponse>()
    }
}
