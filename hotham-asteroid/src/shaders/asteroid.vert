#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_EXT_multiview : enable

layout(binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view[2];
    mat4 projection[2];
    float deltaTime;
    vec4 lightPos;
} ubo;

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;
layout(location = 2) in vec2 inTextureCoordinates;
layout(location = 3) in vec3 inNormal;

layout(location = 0) out vec3 outColor;
layout(location = 1) out vec2 outTextureCoordinates;
layout(location = 2) out vec3 outNormal;
layout(location = 3) out vec3 outViewVec;
layout(location = 4) out vec3 outLightVec;

void main() {
    gl_Position = ubo.projection[gl_ViewIndex] * ubo.view[gl_ViewIndex] * ubo.model * vec4(inPosition, 1.0);
    outColor = inColor;
    outNormal = mat3(ubo.view[gl_ViewIndex]) * inNormal;
    outTextureCoordinates = inTextureCoordinates;

    vec3 lightPos = mat3(ubo.view[gl_ViewIndex] * ubo.model) * ubo.lightPos.xyz;
    vec4 pos = ubo.view[gl_ViewIndex] * vec4(inPosition, 1.0);
    outLightVec = lightPos - pos.xyz;
    outViewVec = -pos.xyz;
}