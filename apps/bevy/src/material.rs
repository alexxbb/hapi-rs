use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, Image, ImageFormat, ImageSampler, ImageType};
use hapi_rs::node::{HoudiniNode, Parameter};
use hapi_rs::parameter::ParmBaseTrait;
use hapi_rs::Result;

struct MaterialNodeParameters {
    color: Parameter,
    normal: Parameter,
    specular: Parameter,
}

fn find_material_nodes(asset: &HoudiniNode) -> Result<MaterialNodeParameters> {
    let session = &asset.session;
    let Parameter::String(color_parm) = asset.parameter("color_map")? else {
        return Err("color_map parm not found".into());
    };

    let Parameter::String(normal_parm) = asset.parameter("normal_map")? else {
        return Err("normal_map parm not found".into());
    };

    let Parameter::String(specular_parm) = asset.parameter("specular_map")? else {
        return Err("specular_map parm not found".into());
    };

    let mat_node_color_parm = session
        .find_parameter_from_path(color_parm.get(0)?, asset)?
        .unwrap();

    let mat_node_normal_parm = session
        .find_parameter_from_path(normal_parm.get(0)?, asset)?
        .unwrap();

    let mat_node_specular_parm = session
        .find_parameter_from_path(specular_parm.get(0)?, asset)?
        .unwrap();

    Ok(MaterialNodeParameters {
        color: mat_node_color_parm,
        normal: mat_node_normal_parm,
        specular: mat_node_specular_parm,
    })
}

fn render_material_texture(parameter: &Parameter) -> Result<Image> {
    let session = parameter.session();
    let node = parameter.node();

    session.render_texture_to_image(node, &parameter.name()?)?;

    let image_info = session.get_image_info(node)?;
    let (width, height) = (image_info.x_res() as usize, image_info.y_res() as usize);
    let mut pixels = Vec::with_capacity(width * height);
    session.extract_image_to_memory(node, &mut pixels, "C", "PNG")?;

    let image = Image::from_buffer(
        &pixels,
        ImageType::Format(ImageFormat::Png),
        CompressedImageFormats::all(),
        true,
        ImageSampler::default(),
        RenderAssetUsages::default(),
    )
    .expect("Could not create Image");
    Ok(image)
}

pub struct TextureMaps {
    pub color: Option<Image>,
    pub normal: Option<Image>,
    pub specular: Option<Image>,
}

pub fn extract_texture_maps(asset: &HoudiniNode, normals: bool) -> Result<TextureMaps> {
    let mat_parms = find_material_nodes(asset)?;
    Ok(TextureMaps {
        color: Some(render_material_texture(&mat_parms.color)?),
        normal: normals.then(|| render_material_texture(&mat_parms.normal).unwrap()), // Houdini normals maps are not compatible with Bevy's default shader
        specular: Some(render_material_texture(&mat_parms.specular)?),
    })
}
