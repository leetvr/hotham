#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_EXT_multiview : enable


// Scene Uniform Buffer
layout(set = 0, binding = 0) uniform UniformBufferObject {
    mat4 view[2];
    mat4 projection[2];
    float deltaTime;
    vec4 lightPos;
} ubo;

// Skin SSBO
layout(std430, set = 1, binding = 0) readonly buffer JointMatrices {
    mat4 jointMatrices[];
};

layout(push_constant) uniform PushConsts {
	mat4 model;
} pushConsts;


layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec2 inTextureCoordinates;
layout(location = 2) in vec3 inNormal;
layout(location = 3) in vec4 inTangent;
layout(location = 4) in vec4 inJointIndices;
layout(location = 5) in vec4 inJointWeights;

layout(location = 0) out vec2 outTextureCoordinates;
layout(location = 1) out vec3 outNormal;
layout(location = 2) out vec3 outViewVec;
layout(location = 3) out vec3 outLightVec;
layout(location = 4) out vec4 outTangent;

void main() {
    mat4 skinMat = 
        inJointWeights.x * jointMatrices[int(inJointIndices.x)] +
        inJointWeights.y * jointMatrices[int(inJointIndices.y)] +
        inJointWeights.z * jointMatrices[int(inJointIndices.z)] +
        inJointWeights.w * jointMatrices[int(inJointIndices.w)];

    gl_Position = ubo.projection[gl_ViewIndex] * ubo.view[gl_ViewIndex] * pushConsts.model * skinMat * vec4(inPosition, 1.0);

    outNormal = normalize(transpose(inverse(mat3(ubo.view[gl_ViewIndex] * pushConsts.model * skinMat))) * inNormal);
    outTextureCoordinates = inTextureCoordinates;

    vec4 pos = ubo.view[gl_ViewIndex] * vec4(inPosition, 1.0);
    vec3 lightPos = mat3(ubo.view[gl_ViewIndex]) * ubo.lightPos.xyz;
    outLightVec = lightPos- pos.xyz;
    outViewVec = -pos.xyz;
    outTangent = inTangent;
}