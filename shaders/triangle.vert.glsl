#version 420 core

in layout (location = 0) vec3 vp;


void main() {
    gl_Position = vec4(vp, 1.0);
}

