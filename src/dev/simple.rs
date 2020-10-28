extern crate hapi_rs as he;

use std::ptr::{null};

// type null_char = *const std::os::raw::c_char;

unsafe fn main_() {
    let cook_options = he::CookOptionsBuilder::new().split_geos_by_attribute(true).build();
    // let mut session = std::mem::MaybeUninit::uninit();
    // he::HAPI_CreateInProcessSession(session.as_mut_ptr());
    // let session = session.assume_init();
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
}

fn main() {
    unsafe { main_() }
}