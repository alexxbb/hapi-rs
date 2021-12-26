#![allow(dead_code)]
use hapi_rs::Result;
use hapi_rs::session::*;

fn main() -> Result<()> {
    new_in_process()?;
    Ok(())
}
