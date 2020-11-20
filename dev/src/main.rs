#![allow(unused)]
// mod dev_1;
mod dev_2;
// mod static_session;
// mod dev_3;

// use static_session::run;
use dev_2::run;
// use dev_3::run;
fn main() {
    if let Err(e) = unsafe{run()} {
        eprintln!("Error: {}", e)
    }
}
