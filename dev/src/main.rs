
// pub trait HapiNode {
//     fn node_type(&self);
//     fn as_any(&self) -> &dyn Any;
// }

#[non_exhaustive]
pub enum HoudiniNode {
    SopNode(SopNode),
    ObjNode(ObjNode),
}

impl HoudiniNode {
}

pub struct SopNode{}
pub struct ObjNode{}

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

fn create_node() -> HoudiniNode {
    HoudiniNode::SopNode(SopNode{})
}

fn main() {
    let nodes = vec![create_node()];
    for n in nodes {
        if let HoudiniNode::SopNode(sop) = n {
           sop.sop_method()
        }
    }
}
