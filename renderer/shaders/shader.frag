#version 450


layout (set = 1, binding = 0) uniform sampler2D tex_sampler;

// layout (location = 0) in vec3 vColor;
layout (location = 0) in vec2 v_tex_coord;

layout (location = 0) out vec4 f_color;

// #define PI 3.1415926538

void main() {
    f_color = texture(tex_sampler, v_tex_coord);
}
