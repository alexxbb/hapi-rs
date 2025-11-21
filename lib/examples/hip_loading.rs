use hapi_rs::session::*;
use std::thread::sleep;
use std::time::Duration;
use tempfile::NamedTempFile;

/// Demonstrates how to save and load Hip files.
fn main() -> Result<()> {
    let sess = new_thrift_session(
        SessionOptions::builder().threaded(true).build(),
        ServerOptions::shared_memory(),
    )?;

    println!("Generating scene");
    for _ in 0..10 {
        // create some nodes
        sess.create_node("Object/simplebiped")?;
    }

    let hip_file = NamedTempFile::with_suffix(".hip")?;
    println!("Saving scene to {}", hip_file.path().display());
    sess.save_hip(hip_file.path(), false)?;

    println!("Cleaning up");
    sess.cleanup()?;
    sess.initialize()?;

    println!("Loading scene {}", hip_file.path().display());
    sess.load_hip(hip_file.path(), true)?;

    // Because the server is in threaded mode, hip loading is asynchronous, and we must poll the session for status.
    loop {
        sleep(Duration::from_millis(100));
        let state = sess.get_cook_state_status()?;
        match state {
            SessionState::StartingLoad => {
                println!("Starting loading");
            }
            SessionState::Loading => {
                println!("Loading...")
            }
            SessionState::Cooking => {
                println!("Cooking assets...")
            }
            SessionState::Ready => {
                println!("Scene loaded!");
                break;
            }
            SessionState::ReadyWithFatalErrors => {
                let error = sess.get_cook_result_string(StatusVerbosity::Statusverbosity2)?;
                eprintln!("Fatal errors when loading hip: {error}");
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
