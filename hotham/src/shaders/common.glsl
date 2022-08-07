#extension GL_EXT_nonuniform_qualifier:enable
#extension GL_ARB_separate_shader_objects : enable
#extension GL_EXT_multiview : enable

#define NO_TEXTURE 4294967295
#define NO_SKIN 4294967295
#define MAX_JOINTS 64

struct DrawData {
    mat4 globalFromLocal;
    mat4 localFromGlobal;
    uint materialID;
    uint skinID;
};

struct Material {
    vec4 baseColorFactor;
    vec4 emissiveFactor;
    vec4 diffuseFactor;
    vec4 specularFactor;
    uint workflow;
    uint baseColorTextureID;
    uint physicalDescriptorTextureID;
    uint normalTextureID;
    uint occlusionTextureID;
    uint emissiveTextureID;
    float metallicFactor;
    float roughnessFactor;
    float alphaMask;
    float alphaMaskCutoff;
};

layout(std430, set = 0, binding = 0) readonly buffer DrawDataBuffer {
    DrawData data[];
} drawDataBuffer;

layout(std430, set = 0, binding = 1) readonly buffer MaterialBuffer {
    Material materials[];
} materialBuffer;

layout(std430, set = 0, binding = 2) readonly buffer SkinsBuffer {
    mat4 jointMatrices[100][64]; // dynamically sized array of 64 element long arrays of mat4.
} skinsBuffer;

layout(set = 0, binding = 3) readonly uniform SceneData {
    mat4 viewProjection[2];
    vec4 cameraPosition[2];
    vec4 lightDirection;
    vec4 debugData;
} sceneData;

// Textures
layout(set = 0, binding = 4) uniform sampler2D textures[];
