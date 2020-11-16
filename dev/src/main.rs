mod dev_1;
mod dev_2;
mod static_session;

use static_session::run;

fn main() {
    if let Err(e) = unsafe{run()} {
        eprintln!("Error: {}", e)
    }
}
