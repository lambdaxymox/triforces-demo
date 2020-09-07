use gdmath::Vector3;


pub struct PointLight {
    pub ambient: Vector3<f32>,
    pub diffuse: Vector3<f32>,
    pub specular: Vector3<f32>,
    pub specular_exponent: f32,
    pub position: Vector3<f32>,
}

impl PointLight {
    pub fn new(
        ambient: Vector3<f32>, diffuse: Vector3<f32>, specular: Vector3<f32>,
        specular_exponent: f32,
        position: Vector3<f32>) -> PointLight {

        PointLight {
            ambient: ambient,
            diffuse: diffuse,
            specular: specular,
            specular_exponent: specular_exponent,
            position: position,
        }
    }
}
