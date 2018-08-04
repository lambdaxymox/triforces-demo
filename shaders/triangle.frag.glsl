#version 420 core

in vec2 tex_coord;
in vec3 normal;
uniform sampler2D tex;
out vec4 frag_color;


void main() {
    frag_color = texture (tex, tex_coord);
}
