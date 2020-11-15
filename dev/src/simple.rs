extern crate hapi_rs as he;

use he::char_ptr;
use he::ffi;
use std::mem::MaybeUninit;
use he::errors::{Result, HapiError, Kind};
use std::ffi::{CStr, CString};

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
                ffi::HAPI_GetAvailableAssetCount(session.const_ptr(), lib_id, &mut num_assets as *mut _);
                println!("Num assets: {}", num_assets);
                let mut names = -1;
                let r =
                    ffi::HAPI_GetAvailableAssets(session.const_ptr(), lib_id, &mut names as *mut _, 1);
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
