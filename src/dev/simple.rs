extern crate hapi_rs as he;

use std::ptr::{null};


fn main_() -> he::Result<()> {
    let cook_options = he::CookOptions::default();
    let session = he::Session::new_in_process()?;
    let mut res = he::Initializer::new();
    res.set_houdini_env_files(&["/foo", "/bar"]);
    res.initialize()?;
    dbg!(session);
    Ok(())
}

fn main() {
    if let Err(_) = main_() {
        eprintln!("{}", he::HAPI_Error::error_string(None))
    }
}