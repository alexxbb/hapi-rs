pub use crate::enums::ImagePacking;

#[derive(Debug, Copy, Clone)]
pub struct CopImageDescription<'a> {
    pub width: u32,
    pub height: u32,
    pub flip_x: bool,
    pub flip_y: bool,
    pub packing: ImagePacking,
    pub image_data: &'a [f32],
}
