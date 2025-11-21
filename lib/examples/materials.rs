use hapi_rs::Result;
/// Example extracts material textures to files
use hapi_rs::geometry::Materials;
use hapi_rs::parameter::*;
use hapi_rs::session::{ServerOptions, SessionOptions, new_thrift_session};

fn main() -> Result<()> {
    let session = new_thrift_session(SessionOptions::default(), ServerOptions::default())?;
    let lib = session.load_asset_file("../otls/sesi/SideFX_spaceship.hda")?;
    let node = lib.try_create_first()?;
    node.cook()?;
    let geo = node.geometry()?.unwrap();
    let part = geo.part_info(0)?;
    let material = match geo.get_materials(&part)?.unwrap() {
        Materials::Single(mat) => mat,
        Materials::Multiple(_) => panic!("All materials should be the same"),
    };
    let mat_node = material.node()?;
    let node_path = node.path()?;
    println!("Material node: {node_path}");

    if let Parameter::String(p) = mat_node.parameter("baseColorMap")? {
        println!("Base color map path: {}", p.get(0)?);
    }
    material.render_texture("baseColorMap")?;

    let image_info = material.get_image_info()?;
    let (x, y) = (image_info.x_res(), image_info.y_res());
    let format = image_info.image_format(&session)?;
    println!("Image [Width x Height] = [{x} x {y}]\nImage Format = {format}");

    for ip in material.get_image_planes()? {
        println!("Image plane: {ip}");
        let tmp_file = std::env::temp_dir().join(format!("spaceship_map_{ip}.jpeg"));
        material.extract_image_to_file(ip, &tmp_file)?;
        println!("Rendered: {}", tmp_file.to_string_lossy());
    }
    Ok(())
}
