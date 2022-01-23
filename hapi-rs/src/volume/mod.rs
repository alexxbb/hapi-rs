mod volume;

pub(crate) use volume::*;

#[derive(Debug, Clone, Default)]
pub struct VolumeBounds {
    pub x_min: f32,
    pub y_min: f32,
    pub z_min: f32,
    pub x_max: f32,
    pub y_max: f32,
    pub z_max: f32,
    pub x_center: f32,
    pub y_center: f32,
    pub z_center: f32,
}
