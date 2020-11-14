use std::sync::Arc;
use crate::auto::bindings as ffi;
use crate::session::Session;
use super::errors::*;

#[derive(Debug)]
#[non_exhaustive]
pub enum HoudiniNode {
    SopNode(SopNode),
    ObjNode(ObjNode),
}

impl HoudiniNode {
    pub fn delete_node(self) -> Result<()> {
        use HoudiniNode::*;
        let (id, session) = match &self {
            SopNode(n) => (n.id, n.session.ffi_ptr()),
            ObjNode(n) => (n.id, n.session.ffi_ptr()),
        };
        unsafe {
            let res = ffi::HAPI_DeleteNode(session, id);
            hapi_ok!(res, session)
        }
    }
}

#[derive(Debug)]
pub struct SopNode {
    id: ffi::HAPI_NodeId,
    session: Arc<Session>
}
#[derive(Debug)]
pub struct ObjNode{
    id: ffi::HAPI_NodeId,
    session: Arc<Session>
}

impl SopNode {
    fn sop_method(&self) {
        println!("I'm a sop node")
    }
}

impl ObjNode {
    fn obj_method(&self) {
        println!("I'm an obj node")
    }
}
