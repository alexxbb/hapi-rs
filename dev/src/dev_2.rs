extern crate hapi_rs as he;

use self::he::State;
use he::char_ptr;
use he::errors::{HapiError, Kind, Result};
use he::ffi;
use he::session::SessionHandle;
use smol;
use std::ffi::CString;
use std::future::Future;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::ptr::null;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

async fn ping() {
    smol::Timer::after(std::time::Duration::from_millis(150)).await;
    println!("Ping!");
}

async fn pong() {
    smol::Timer::after(std::time::Duration::from_millis(160)).await;
    println!("Pong!");
}

struct CookFuture {
    node_id: i32,
    session: Arc<SessionHandle>,
}

impl CookFuture {
    fn cook(node_id: i32, session: Arc<SessionHandle>) -> CookFuture {
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
            s => {
                Poll::Ready(Err(()))
            }
        }
    }
}

pub unsafe fn run() -> Result<()> {
    let session = he::session::Session::new_in_process()?;
    session.initialize()?;
    let otl = char_ptr!("/Users/alex/sandbox/rust/hapi/otls/sleeper.hda");
    let mut lib_id = MaybeUninit::uninit();
    let r = he::ffi::HAPI_LoadAssetLibraryFromFile(
        session.ffi_ptr(),
        otl,
        false as i8,
        lib_id.as_mut_ptr(),
    );
    assert!(matches!(r, ffi::HAPI_Result::HAPI_RESULT_SUCCESS));

    let lib_id = lib_id.assume_init();
    let mut num_assets = -1;
    ffi::HAPI_GetAvailableAssetCount(session.ffi_ptr(), lib_id, &mut num_assets as *mut _);
    let mut names = -1;
    let r = ffi::HAPI_GetAvailableAssets(session.ffi_ptr(), lib_id, &mut names as *mut _, 1);
    let names = std::slice::from_raw_parts(&names as *const _, 1);
    let asset_name = he::get_string(names[0], session.ffi_ptr())?;
    let mut id = MaybeUninit::uninit();
    ffi::HAPI_CreateNode(
        session.ffi_ptr(),
        -1,
        CString::from_vec_unchecked(asset_name.into_bytes()).as_ptr(),
        char_ptr!("Sleeper"),
        0i8,
        id.as_mut_ptr(),
    );
    let id = id.assume_init();
    let cook = CookFuture::cook(id, session.handle().clone());
    smol::spawn(async {
        for _ in 0..10 {
            ping().await;
        }
    }).detach();
    smol::spawn(async move {
        loop {
            match cook.state() {
                State::StateReady => {println!("Cooking done"); return},
                State::StateCooking => {
                    println!("Waiting");
                    smol::Timer::after(Duration::from_millis(100)).await;
                },
                _ => ()
            };
        }
    }).detach();
    // std::thread::sleep(std::time::Duration::from_secs(8));
    Ok(())
}
