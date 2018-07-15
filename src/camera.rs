use math;
use math::{Vector3, Vector4, Matrix4, Quaternion};


pub struct Camera {
    // Camera parameters.
    pub near: f32,
    pub far: f32,
    pub fov: f32,
    pub aspect: f32,

    // Camera kinematics.
    pub speed: f32,
    pub yaw_speed: f32,
    pub pos: Vector3,
    pub fwd: Vector4,
    pub rgt: Vector4,
    pub up: Vector4,
    pub axis: Quaternion,

    // Camera matrices.
    pub proj_mat: Matrix4,
    pub trans_mat: Matrix4,
    pub rot_mat: Matrix4,
    pub view_mat: Matrix4,
}

impl Camera {
    pub fn new(
        near: f32, far: f32, fov: f32, aspect: f32,
        cam_speed: f32, cam_yaw_speed: f32, cam_pos: Vector3,
        fwd: Vector4, rgt: Vector4, up: Vector4, axis: Quaternion) -> Camera {

        let proj_mat = math::perspective((fov, aspect, near, far));
        let trans_mat = Matrix4::from_translation(-cam_pos);
        let rot_mat = Matrix4::from(axis);
        let view_mat = rot_mat * trans_mat;

        Camera {
            near: near,
            far: far,
            fov: fov,
            aspect: aspect,

            speed: cam_speed,
            yaw_speed: cam_yaw_speed,
            pos: cam_pos,
            fwd: fwd,
            rgt: rgt,
            up: up,
            axis: axis,

            proj_mat: proj_mat,
            trans_mat: trans_mat,
            rot_mat: rot_mat,
            view_mat: view_mat,
        }
    }
}

