#version 450

layout (location = 0) in vec3 vColor;

layout (location = 0) out vec4 fColor;

// #define PI 3.1415926538

void main() {
    fColor = vec4(vColor, 1.0);
}
