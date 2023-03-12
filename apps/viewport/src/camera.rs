use ultraviolet::{Mat4, Vec3};

#[derive(Copy, Clone)]
pub struct Camera {
    eye: Vec3,
    look_at: Vec3,
    up_vec: Vec3,
    view: Mat4,
    aspect_ratio: f32,
    zoom: f32,
}

impl Camera {
    pub fn new(eye: Vec3) -> Self {
        let up_vec = Vec3::unit_y();
        let look_at = Vec3::zero();
        let view = Mat4::look_at(eye, look_at, up_vec);
        Camera {
            eye,
            look_at,
            up_vec,
            view,
            aspect_ratio: 1.0,
            zoom: 1.0,
        }
    }
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        let rot_y = -delta_x / 150.0;
        let rot_mat = Mat4::from_rotation_y(rot_y);
        self.eye = rot_mat.transform_vec3(self.eye);
        self.view = Mat4::look_at(self.eye, self.look_at, self.up_vec);

        let eye_dir = (self.look_at - self.eye).normalized();
        let ortho = eye_dir.cross(self.up_vec);

        let rot_ortho = -delta_y / 150.0;
        let rot_mat = Mat4::from_rotation_around(ortho.into_homogeneous_vector(), rot_ortho);

        let eye_local = rot_mat.transform_vec3(self.eye - self.look_at);

        let new_eye = eye_local + self.look_at;
        let new_view_dir = self.look_at - new_eye;

        let cos_angle = new_view_dir.dot(self.up_vec) / (new_view_dir.mag() * self.up_vec.mag());

        if cos_angle < 0.95 && cos_angle > -0.95 {
            self.eye = eye_local + self.look_at;
            self.view = Mat4::look_at(self.eye, self.look_at, self.up_vec);
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        self.view
    }

    pub fn projection_matrix(&self) -> Mat4 {
        ultraviolet::projection::perspective_gl(45.0, self.aspect_ratio, 0.01, 20.0)
    }
    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        let dir = (self.look_at - self.eye).normalized();
        self.eye += dir * zoom;
    }

    pub fn position(&self) -> Vec3 {
        self.eye
    }
}
