extern crate hapi_rs as he;

use self::he::State;
use he::char_ptr;
use he::errors::{Result};
use he::ffi;
use he::session::SessionHandle;
use smol;
use std::ffi::CString;

use std::mem::MaybeUninit;
use std::pin::Pin;
use std::ptr::null;
use std::task::{Context, Poll};
use std::time::Duration;

struct CookFuture {
    node_id: i32,
    session: SessionHandle,
}

impl CookFuture {
    fn cook(node_id: i32, session: SessionHandle) -> CookFuture {
        unsafe {
            let r = ffi::HAPI_CookNode(session.ffi_ptr(), node_id, null());
            assert!(matches!(r, ffi::HAPI_Result::HAPI_RESULT_SUCCESS));
        }
        CookFuture { node_id, session }
    }

    fn state(&self) -> State {
        let status = unsafe {
            let mut status = MaybeUninit::uninit();
            ffi::HAPI_GetStatus(
                self.session.ffi_ptr(),
                ffi::HAPI_StatusType::HAPI_STATUS_COOK_STATE,
                status.as_mut_ptr(),
            );
            status.assume_init()
        };
        State::from(status)
    }
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

pub unsafe fn run() -> Result<()> {
    let session = he::session::Session::new_in_process()?;
    session.initialize()?;
    let library = session.load_asset_file("/Users/alex/sandbox/rust/hapi/otls/sleeper.hda")?;
    let names = library.get_asset_names()?;
    println!("{:?}", names);

    // let mut id = MaybeUninit::uninit();
    // ffi::HAPI_CreateNode(
    //     session.handle().ffi_ptr(),
    //     -1,
    //     CString::from_vec_unchecked(names[0].clone().into_bytes()).as_ptr(),
    //     char_ptr!("Sleeper"),
    //     0i8,
    //     id.as_mut_ptr(),
    // );
    // let id = id.assume_init();
    // let cook = CookFuture::cook(id, session.handle().clone());
    // smol::spawn(async {
    //     for _ in 0..10 {
    //         ping().await;
    //     }
    // }).detach();
    // smol::spawn(async move {
    //     loop {
    //         match cook.state() {
    //             State::StateReady => {println!("Cooking done"); return},
    //             State::StateCooking => {
    //                 println!("Waiting");
    //                 smol::Timer::after(Duration::from_millis(100)).await;
    //             },
    //             _ => ()
    //         };
    //     }
    // }).detach();
    // std::thread::sleep(std::time::Duration::from_secs(8));
    Ok(())
}
