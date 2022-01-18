/// Example extracts material textures to files
use hapi_rs::geometry::Materials;
use hapi_rs::parameter::*;
use hapi_rs::session::quick_session;
use hapi_rs::Result;

fn main() -> Result<()> {
    let session = quick_session(None)?;
    let lib = session.load_asset_file("otls/sesi/SideFX_spaceship.otl")?;
    let node = lib.try_create_first()?;
    node.cook(None)?;
    let geo = node.geometry()?.unwrap();
    let material = match geo.get_materials(None)?.unwrap() {
        Materials::Single(mat) => mat,
        Materials::Multiple(_) => panic!("All materials should be the same"),
    };
    let mat_node = material.node()?;
    let node_path = node.path(None)?;
    println!("Material node: {node_path}");

    if let Parameter::String(p) = mat_node.parameter("baseColorMap")? {
        let values = p.get_value()?;
        let path = &values[0];
        println!("Base color map path: {path}");
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
