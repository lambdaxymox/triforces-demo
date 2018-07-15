#version 420 core

in vec2 st;
uniform sampler2D tex;
out vec4 frag_color;


void main() {
    frag_color = vec4 (0.5, 0.0, 0.5, 1.0);
}
