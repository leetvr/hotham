#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_EXT_multiview : enable

layout(binding = 0) uniform UniformBufferObject {
    mat4 mvp[2];
    float deltaTime;
} ubo;

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;

layout(location = 0) out vec3 outColor;

void main() {
    gl_Position = ubo.mvp[gl_ViewIndex] * vec4(inPosition, 1.0);
    outColor = inColor;
}