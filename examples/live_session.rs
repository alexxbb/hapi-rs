use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, Result};

use hapi_rs::session::{connect_to_pipe, start_houdini_server};

const PIPE: &str = "hapi";

fn main() -> Result<()> {
    let session = match connect_to_pipe(PIPE, None, None) {
        Ok(session) => session,
        Err(_) => {
            let hfs = std::env::var_os("HFS").ok_or_else(|| anyhow!("Missing HFS"))?;
            start_houdini_server(PIPE, Path::new(&hfs).join("bin").join("houdini"))?;
            connect_to_pipe(PIPE, None, Some(Duration::from_secs(10)))?
        }
    };

    let _geo = session.create_node("Object/geo")?;

    Ok(())
}
