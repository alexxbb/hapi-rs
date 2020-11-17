extern crate hapi_rs as he;
use std::error::Error;
use he::session::Session;

pub unsafe fn run() -> std::result::Result<(), Box<dyn Error>> {
    let ses = Session::new_in_process()?;
    ses.initialize()?;
    ses.create_node("Object/geo", None, None)?;
    ses.save_hip("/tmp/foo.hip")?;
    Ok(())
}
