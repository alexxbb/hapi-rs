#![allow(dead_code)]
use hapi_rs::session::new_in_process;
use hapi_rs::Result;

fn main() -> Result<()> {
    new_in_process()?;
    Ok(())

}
