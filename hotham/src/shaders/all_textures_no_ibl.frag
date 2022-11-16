#version 460
#extension GL_EXT_nonuniform_qualifier:enable
#extension GL_ARB_separate_shader_objects : enable

// Inputs
layout (location = 0) in vec4 inGosPos;
layout (location = 1) in vec2 inUV;
layout (location = 2) flat in uint inMaterialID;
layout (location = 3) in vec3 inNormal;

// Textures
layout (set = 0, binding = 4) uniform sampler2D textures[];
layout (set = 0, binding = 5) uniform samplerCube cubeTextures[];

struct Material {
    uint baseColorTextureID;
    uint metallicRoughnessTextureID;
    uint normalTextureID;
    uint emissiveTextureID;
};

// layout (set = 1, binding = 0) uniform sampler2D baseColorTexture;

layout( push_constant ) uniform constants
{
    uint baseColorTextureID;
} PushConstants;

// Outputs
layout (location = 0) out vec4 outColor;

// The world's worst shader (TM).
//
// Takes all the textures and blends them into a slurry. Gives no fucks whether the texture is actually there.
void main() {
    // Retrieve the material from the buffer.
    // Material material = materialBuffer.materials[inMaterialID];
    // vec4 textureID = materialBuffer.textureIDs[inMaterialID];
    uint start = PushConstants.baseColorTextureID;
    vec4 baseColor = texture(textures[start], inUV);
    vec4 mr = texture(textures[start + 1], inUV);
    vec4 normal =   texture(textures[start + 2], inUV);
    vec4 emissive = texture(textures[start + 3], inUV);
    // vec4 baseColor = texture(textures[4], inUV);
    // vec4 mr = texture(textures[5], inUV);
    // vec4 normal =   texture(textures[6], inUV);
    // vec4 emissive = texture(textures[7], inUV);
    // outColor = textureID;
    // outColor = texture(baseColorTexture, inUV);
    // vec4 baseColor = texture(textures[PushConstants.baseColorTextureID], inUV);
    // outColor = baseColor;
    outColor = baseColor + mr + normal + emissive;
}