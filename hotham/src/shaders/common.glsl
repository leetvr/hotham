#extension GL_EXT_nonuniform_qualifier:enable
#extension GL_ARB_separate_shader_objects : enable
#extension GL_EXT_multiview : enable

#define NO_TEXTURE 4294967295

struct DrawData {
    mat4 transform;
    vec4 boundingSphere;
    uint materialID;
    vec3 pad;
};

struct Material {
    vec4 baseColorFactor;
    vec4 emissiveFactor;
    vec4 diffuseFactor;
    vec4 specularFactor;
    uint workflow;
    uint baseColorTextureID;
    uint metallicRoughnessTextureID;
    uint normalTextureID;
    uint occlusionTextureID;
    uint emissiveTextureID;
    float metallicFactor;
    float roughnessFactor;
    float alphaMask;
    float alphaMaskCutoff;
};

struct VkDrawIndexedIndirectCommand
{
	uint indexCount;
	uint instanceCount;
	uint firstIndex;
	int  vertexOffset;
	uint firstInstance;
};

struct Light {
    vec4 position;
    vec4 color;
    vec4 direction;
};

layout (push_constant) uniform SceneData { 
    mat4 viewProjection[2];
    vec4 cameraPosition[2];
    vec4 lightDirection;
    vec4 debugData;
} sceneData;

layout(std140, set = 0, binding = 0) readonly buffer DrawDataBuffer {
    DrawData data[];
} drawDataBuffer;

layout(std140, set = 0, binding = 1) readonly buffer MaterialBuffer {
    Material materials[];
} materialBuffer;

layout(std430, set = 0, binding = 2) writeonly buffer DrawCommandsBuffer {
    VkDrawIndexedIndirectCommand drawCommands[];
} draw_commands_buffer;

// Textures
layout(set = 0, binding = 3) uniform sampler2D textures[];