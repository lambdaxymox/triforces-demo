use math::Vector3;


struct PointLight {
    specular: f32,
    diffuse: f32,
    ambient: f32,
    position: Vector3,
}

impl PointLight {
    fn new(specular: f32, diffuse: f32, ambient: f32, position: Vector3) -> PointLight {
        PointLight {
            specular: specular,
            diffuse: diffuse,
            ambient: ambient,
            position: position,
        }
    }
}
