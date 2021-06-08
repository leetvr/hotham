#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;
layout(location = 0) out vec3 outColor;

void main() {
    gl_Position = vec4(inPosition, 0.0);
    outColor = inColor;
}