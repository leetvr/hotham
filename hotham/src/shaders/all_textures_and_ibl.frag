#version 460
#extension GL_EXT_nonuniform_qualifier:enable
#extension GL_ARB_separate_shader_objects : enable
#define BRDF_LUT_TEXTURE_ID 0
#define SAMPLER_IRRADIANCE_TEXTURE_ID 0
#define ENVIRONMENT_MAP_TEXTURE_ID 1

// Inputs
layout (location = 0) in vec3 inGosPos;
layout (location = 1) in vec2 inUV;
layout (location = 2) flat in uint inMaterialID;
layout (location = 3) in vec3 inNormal;

// Textures
layout (set = 0, binding = 4) uniform sampler2D textures[];
layout (set = 0, binding = 5) uniform samplerCube cubeTextures[];

struct Material {
    vec4 baseColorFactor;
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

layout (std430, set = 0, binding = 1) readonly buffer MaterialBuffer {
    Material materials[];
} materialBuffer;

// Outputs
layout (location = 0) out vec4 outColor;

// The world's worst shader (TM).
//
// Takes all the textures and blends them into a slurry. Gives no fucks whether the texture is actually there.
void main() {
    // Retrieve the material from the buffer.
    Material material = materialBuffer.materials[inMaterialID];
    outColor = texture(textures[material.baseColorTextureID], inUV) +
        texture(textures[material.metallicRoughnessTextureID], inUV) +
        texture(textures[material.normalTextureID], inUV) +
        texture(textures[material.emissiveTextureID], inUV) +
        texture(textures[BRDF_LUT_TEXTURE_ID], inUV) +
        textureLod(cubeTextures[ENVIRONMENT_MAP_TEXTURE_ID], inUV.xyy, 1) +
        textureLod(cubeTextures[SAMPLER_IRRADIANCE_TEXTURE_ID], inUV.xyy, 1);
}
