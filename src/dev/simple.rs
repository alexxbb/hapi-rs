extern crate hapi_rs as he;

use std::mem::MaybeUninit;
use he::char_ptr;


fn main_() -> he::Result<()> {
    let cook_options = he::CookOptions::default();
    let session = he::Session::new_in_process()?;
    let mut res = he::Initializer::new(session.clone());
    res.set_cook_options(&cook_options);
    res.set_houdini_env_files(&["/foo", "/bar"]);
    res.initialize()?;
    unsafe {
        let otl = char_ptr!("/net/soft_scratch/users/arusev/rust/hapi-rs/examples/otls/spaceship.otl");
        let mut lib_id = MaybeUninit::uninit();
        let r = he::ffi::HAPI_LoadAssetLibraryFromFile(
          session.const_ptr(),
            otl,
            false as i8,
            lib_id.as_mut_ptr()
        );

        match r {
            he::ffi::HAPI_Result::HAPI_RESULT_SUCCESS => {
                let hip = char_ptr!("/mcp/foo.hip");
                let r = he::ffi::HAPI_SaveHIPFile(session.const_ptr(), hip, true as i8);
                assert!(matches!(r, he::ffi::HAPI_Result::HAPI_RESULT_SUCCESS));
            },
            e => {
                let e = he::HAPI_Error::new(e, session.const_ptr());
                println!("{}", e);
            }
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = main_() {
        eprintln!("{}", e)
    }
}