extern crate hapi_rs as he;

use std::ptr::{null};

// type null_char = *const std::os::raw::c_char;

fn main_() -> he::Result<()> {
    let cook_options = he::CookOptions::default();
    let session = he::Session::new_in_process()?;
    let mut res = he::Initializer::new();
    res.set_houdini_env_files(&["/foo", "/bar"]);
    res.initialize()?;
    dbg!(session);
    // let res = he::HAPI_Initialize(
    //     &session as *const _,
    //     &cook_options as *const _,
    //     1,
    //     -1,
    //     null(),
    //     null(),
    //     null(),
    //     null(),
    //     null(),
    // );
    //
    // let r = he::HAPI_Cleanup(&session as *const _);
    Ok(())
}

fn main() {
    if let Err(_) = main_() {
        eprintln!("{}", he::HAPI_Error::error_string(None))
    }
}