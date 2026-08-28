use crate::{SessionTreeCommand, SessionTreeResponse, SESSION_TREE_SERVICE};
use phenix_core::{ComponentInterface, InterfaceId};

pub struct SessionTreeInterface;

impl ComponentInterface for SessionTreeInterface {
    type Request = SessionTreeCommand;
    type Response = SessionTreeResponse;

    fn interface_id() -> InterfaceId {
        InterfaceId::parse(SESSION_TREE_SERVICE).expect("static session-tree interface id is valid")
    }
}
