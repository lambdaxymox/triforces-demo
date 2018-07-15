#version 420 core

in vec3 vp;
uniform mat4 proj_mat, view_mat, model_mat;


void main() {
    gl_Position = proj_mat * view_mat * model_mat * vec4 (vp, 1.0);
}
