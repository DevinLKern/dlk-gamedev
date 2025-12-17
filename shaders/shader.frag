#version 450


layout (set = 1, binding = 0) uniform sampler2D texSampler;

// layout (location = 0) in vec3 vColor;
layout (location = 0) in vec2 vTexCoord;

layout (location = 0) out vec4 fColor;

// #define PI 3.1415926538

void main() {
    fColor = texture(texSampler, vTexCoord);
}
