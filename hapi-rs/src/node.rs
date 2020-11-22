use super::errors::*;
use crate::auto::bindings as ffi;
pub use crate::auto::rusty::NodeType;
use crate::char_ptr;
use crate::session::SessionHandle;
use crate::auto::rusty::State;
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::ptr::null;
use std::sync::Arc;
use std::task::{Poll, Context};
use std::pin::Pin;

#[cfg(feature = "async")]
mod async_ {
    use super::*;
    pub struct CookFuture {
        node_id: i32,
        session: SessionHandle,
    }

    impl CookFuture {
        pub fn new(node_id: i32, session: SessionHandle) -> CookFuture {
            unsafe {
                let r = ffi::HAPI_CookNode(session.ptr(), node_id, null());
                assert!(matches!(r, ffi::HAPI_Result::HAPI_RESULT_SUCCESS));
            }
            CookFuture { node_id, session }
        }

        // pub fn complete(&self) -> std::result::Result<(), ()> {
        //     eprintln!("Starting cooking");
        //     loop {
        //         match self.state() {
        //             State::StateReady => break Ok(()),
        //             State::StateCooking | State::StartingCook => {
        //             }
        //             State::CookErrors => break Err(()),
        //             _s => {}
        //         }
        //     }
        // }
    }

    impl std::future::Future for CookFuture {
        type Output = std::result::Result<State, ()>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            match self.state() {
                State::StateReady => Poll::Ready(Ok(State::StateReady)),
                State::StateCooking | State::StartingCook => {
                    cx.waker().wake_by_ref();
                    eprintln!("Cooking...");
                    Poll::Pending
                }
                State::CookErrors => Poll::Ready(Err(())),
                _s => {
                    Poll::Ready(Err(()))
                }
            }
        }
    }

}
#[derive(Debug)]
#[non_exhaustive]
pub enum HoudiniNode {
    SopNode(SopNode),
    ObjNode(ObjNode),
}

impl HoudiniNode {
    pub fn delete(self) -> Result<()> {
        use HoudiniNode::*;
        let (id, session) = match &self {
            SopNode(n) => (n.id, n.session.ptr()),
            ObjNode(n) => (n.id, n.session.ptr()),
        };
        unsafe {
            let mut info = MaybeUninit::uninit();
            ffi::HAPI_GetNodeInfo(session, id, info.as_mut_ptr()).result(session, None)?;
            let info = info.assume_init();
            // if info.createdPostAssetLoad != 0 {
            //     unimplemented!()
            // }
            ffi::HAPI_DeleteNode(session, id).result(session, None)
        }
    }

    #[inline]
    fn strip(&self) -> (ffi::HAPI_NodeId, &SessionHandle) {
        match &self {
            HoudiniNode::SopNode(n) => (n.id, &n.session),
            HoudiniNode::ObjNode(n) => (n.id, &n.session),
        }
    }

    #[cfg(not(feature = "async"))]
    pub fn cook(&self) -> Result<()> {
        let (id, session) = self.strip();
        debug_assert!(session.sync.get(), "Session is async!");
        unsafe { ffi::HAPI_CookNode(session.ptr(), id, null()).result(session.ptr(), None) }
    }

    #[cfg(feature = "async")]
    pub fn cook(&self) -> async_::CookFuture {
        let (id, session) = self.strip();
        async_::CookFuture::new(id, session.clone())
    }

    pub fn create(
        name: &str,
        label: Option<&str>,
        parent: Option<HoudiniNode>,
        session: SessionHandle,
        cook: bool,
    ) -> Result<HoudiniNode> {
        let mut id = MaybeUninit::uninit();
        let parent = parent.map_or(-1, |n| n.strip().0);
        let mut label_ptr: *const std::os::raw::c_char = null();
        unsafe {
            let mut tmp;
            if let Some(lb) = label {
                tmp = CString::from_vec_unchecked(lb.into());
                label_ptr = tmp.as_ptr();
            }
            let name = CString::from_vec_unchecked(name.into());
            ffi::HAPI_CreateNode(
                session.ptr(),
                parent,
                name.as_ptr(),
                label_ptr,
                cook as i8,
                id.as_mut_ptr(),
            )
            .result(session.ptr(), None)?;
            Ok(HoudiniNode::ObjNode(ObjNode {
                id: id.assume_init(),
                session,
            }))
        }
    }
}

#[derive(Debug)]
pub struct SopNode {
    id: ffi::HAPI_NodeId,
    session: SessionHandle,
}
#[derive(Debug)]
pub struct ObjNode {
    id: ffi::HAPI_NodeId,
    session: SessionHandle,
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
