extern crate hapi_rs as he;
use std::error::Error;
use he::session::Session;

pub unsafe fn run() -> std::result::Result<(), Box<dyn Error>> {
    let ses = Session::new_in_process()?;
    let ses2 = ses.clone();
    let ses3 = ses.clone();
    // ses.initialize()?;
    Ok(())
}
