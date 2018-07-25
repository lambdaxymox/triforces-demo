#version 420 core

in layout (location = 0) vec3 v_pos;
in layout (location = 1) vec2 v_tex;
uniform mat4 proj_mat, view_mat, model_mat;
out vec2 tex_coord;


void main() {
    tex_coord = v_tex;
    gl_Position = proj_mat * view_mat * model_mat * vec4 (v_pos, 1.0);
}
