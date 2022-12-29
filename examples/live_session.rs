/// This example starts an interactive Houdini session first time you run it,
/// and consecutive calls will connect to it and reuse.
use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, Result};
use argh::FromArgs;

use hapi_rs::{
    enums::CurveType,
    geometry::InputCurveInfo,
    session::{connect_to_pipe, start_houdini_server, ManagerType, SessionSyncInfo, Viewport},
};

#[derive(FromArgs, Debug)]
/// Demo of live connection to Houdini.
struct Args {
    /// shape radius. Default 6.0
    #[argh(option, short = 'r', default = "6.0")]
    radius: f32,

    /// curl amplitude. Default 2.0
    #[argh(option, short = 'a', default = "2.0")]
    amplitude: f32,

    /// curl frequency. Default 10.0
    #[argh(option, short = 'f', default = "10.0")]
    frequency: f32,
}

fn main() -> Result<()> {
    let args: Args = argh::from_env();
    const PIPE: &str = "hapi";
    // Try to connect toa possibly running session
    let session = match connect_to_pipe(PIPE, None, None) {
        Ok(session) => session,
        Err(_) => {
            // No session running at PIPE, start the Houdini process.
            // Edit the executable path if necessary.
            let hfs = std::env::var_os("HFS").ok_or_else(|| anyhow!("Missing HFS"))?;
            let executable = Path::new(&hfs).join("bin").join("houdini");
            start_houdini_server(PIPE, executable, true)?;
            connect_to_pipe(PIPE, None, Some(Duration::from_secs(30)))?
        }
    };

    // Set up camera
    session.set_sync(true)?;
    session.set_sync_info(&SessionSyncInfo::default().with_sync_viewport(true))?;
    let vp = Viewport::default()
        .with_position([0.0, 1.0, 20.0])
        .with_rotation([0.0, 0.0, 0.0, 0.0]);
    session.set_viewport(&vp)?;

    // Delete all previous nodes if any.
    for handle in session.get_manager_node(ManagerType::Obj)?.get_children()? {
        session.delete_node(handle)?;
    }

    // Create a curvy shape
    let curve_info = InputCurveInfo::default()
        .with_curve_type(CurveType::Nurbs)
        .with_order(3);
    let geo = session.create_input_curve_node("curvy")?;
    geo.set_input_curve_info(0, &curve_info)?;
    let mut points = Vec::new();
    let radius = args.radius;
    let ampl = args.amplitude;
    for i in 0..100 {
        let t = i as f32 * 0.08;
        let x = t.sin() * radius + (t * args.frequency).cos() * ampl;
        let y = t.cos() * radius + (t * args.frequency).sin() * ampl;
        points.extend_from_slice(&[x, y, 0.0]);
    }
    geo.set_input_curve_positions(0, &points)?;

    Ok(())
}
