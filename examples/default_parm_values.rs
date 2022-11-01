/// Prints hda parameters default values and types
use hapi_rs::session::quick_session;
use hapi_rs::Result;

fn main() -> Result<()> {
    let ses = quick_session(None)?;
    let lib = ses.load_asset_file("otls/sesi/SideFX_spaceship.hda")?;
    let parms = lib.get_asset_parms("SideFX::Object/spaceship")?;
    for p in &parms {
        println!(
            "Parm {} - {:?} - {:?}",
            p.name()?,
            p.parm_type(),
            p.default_value()
        );
    }

    Ok(())
}
