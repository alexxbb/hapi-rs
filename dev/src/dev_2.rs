extern crate hapi_rs as he;

use self::he::State;
use he::char_ptr;
use he::errors::Result;
use he::ffi;
use he::session::{Session, SessionHandle};
use smol;
use std::ffi::CString;

use once_cell::sync;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::ptr::null;
use std::task::{Context, Poll};
use std::time::Duration;

pub unsafe fn run() -> Result<()> {
    let session = Session::new_named_pipe("/tmp/hapi")?;
    session.initialize()?;
    let library = session.load_asset_file("/Users/alex/sandbox/rust/hapi/otls/spaceship.otl")?;
    let names = library.get_asset_names()?;
    let node = session.create_node(&names[0], None, None)?;
    node.cook();
    // let cook = node.cook_async();
    // smol::block_on(smol::unblock(move || cook.complete()));
    // println!("{:?}", names);
    Ok(())
}
