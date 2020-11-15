extern crate hapi_rs as he;

use he::char_ptr;
use he::errors::{HapiError, Kind, Result};
use he::ffi;
use std::ffi::{CStr, CString};
use std::mem::MaybeUninit;
use std::ptr::null;
use self::he::State;

pub fn run() -> Result<()> {
    let session = he::session::Session::new_in_process()?;
    session.initialize()?;
    unsafe {
        let otl = char_ptr!("/Users/alex/sandbox/rust/hapi/otls/sleeper.hda");
        let mut lib_id = MaybeUninit::uninit();
        let r = he::ffi::HAPI_LoadAssetLibraryFromFile(
            session.const_ptr(),
            otl,
            false as i8,
            lib_id.as_mut_ptr(),
        );

        match r {
            ffi::HAPI_Result::HAPI_RESULT_SUCCESS => {
                let lib_id = lib_id.assume_init();
                let mut num_assets = -1;
                ffi::HAPI_GetAvailableAssetCount(
                    session.const_ptr(),
                    lib_id,
                    &mut num_assets as *mut _,
                );
                println!("Num assets: {}", num_assets);
                let mut names = -1;
                let r = ffi::HAPI_GetAvailableAssets(
                    session.const_ptr(),
                    lib_id,
                    &mut names as *mut _,
                    1,
                );
                let names = std::slice::from_raw_parts(&names as *const _, 1);
                let asset_name = he::get_string(names[0], session.const_ptr())?;
                let mut id = MaybeUninit::uninit();
                ffi::HAPI_CreateNode(
                    session.const_ptr(),
                    -1,
                    CString::from_vec_unchecked(asset_name.into_bytes()).as_ptr(),
                    char_ptr!("Sleeper"),
                    0i8,
                    id.as_mut_ptr(),
                );
                let id = id.assume_init();
                let r = ffi::HAPI_CookNode(session.const_ptr(), id, null());
                assert!(matches!(r, ffi::HAPI_Result::HAPI_RESULT_SUCCESS));
                println!("Starting cooking");
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    let mut status = MaybeUninit::uninit();
                    ffi::HAPI_GetStatus(
                        session.const_ptr(),
                        ffi::HAPI_StatusType::HAPI_STATUS_COOK_STATE,
                        status.as_mut_ptr(),
                    );
                    let status = status.assume_init();
                    match he::State::from(status) {
                        he::State::StateCooking => {println!("Cooking")},
                        he::State::StateReady => {
                            println!("Ready!");
                            break
                        },
                        e => {dbg!(e);}
                    }
                }
                println!("Done cooking");

                // let hip = char_ptr!("/tmp/foo.hip");
                // let r = ffi::HAPI_SaveHIPFile(session.const_ptr(), hip, true as i8);
                // assert!(matches!(r, ffi::HAPI_Result::HAPI_RESULT_SUCCESS));
            }
            e => {
                let e = HapiError::new(Kind::Hapi(e), Some(session.const_ptr()));
                println!("{}", e);
            }
        }
    }
    Ok(())
}
