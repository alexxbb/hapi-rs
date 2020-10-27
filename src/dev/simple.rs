extern crate hapi_rs as he;

use std::ptr::null;

// type null_char = *const std::os::raw::c_char;

unsafe fn main_() {
    let cook_options = he::HAPI_CookOptions_Create();
    let c = he::HAPI_Initialize(
        null(),
        &cook_options as *const he::HAPI_CookOptions,
        false as i8,
        -1,
        null(),
        null(),
        null(),
        null(),
        null(),
    );

    he::HAPI_Cleanup(null());
}

fn main() {
    unsafe { main_() }
}