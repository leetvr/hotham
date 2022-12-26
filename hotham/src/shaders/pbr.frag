// PBR shader based on the Khronos glTF-Sample Viewer:
// https://github.com/KhronosGroup/glTF-WebGL-PBR

#version 460
#extension GL_GOOGLE_include_directive : require
#include "common.glsl"
#include "lights.glsl"
#include "brdf.glsl"

// Inputs
layout (location = 0) in vec3 inGosPos;
layout (location = 1) in vec2 inUV;
layout (location = 2) flat in uint inMaterialID;
layout (location = 3) in vec3 inNormal;

// Textures
layout (set = 0, binding = 4) uniform sampler2D textures[];
layout (set = 0, binding = 5) uniform samplerCube cubeTextures[];

#include "pbr.glsl"

layout (std430, set = 0, binding = 1) readonly buffer MaterialBuffer {
    Material materials[];
} materialBuffer;

// Outputs
layout (location = 0) out vec4 outColor;

void main() {
    // Start by setting the output color to a familiar "error" magenta.
    outColor = ERROR_MAGENTA;

    // Retrieve the material from the buffer.
    material = materialBuffer.materials[inMaterialID];
    materialFlags = material.flagsAndBaseTextureID & 0xFFFF;
    baseTextureID = material.flagsAndBaseTextureID >> 16;

    // Determine the base color
    vec4 baseColor = unpackUnorm4x8(material.packedBaseColor);

    if ((materialFlags & HAS_BASE_COLOR_TEXTURE) != 0) {
        baseColor *= texture(textures[baseTextureID], inUV);
    }

    // Choose the correct workflow for this material
    if ((materialFlags & PBR_WORKFLOW_UNLIT) == 0) {
        outColor.rgb = getPBRMetallicRoughnessColor(baseColor);
    } else {
        outColor = baseColor;
    }

    // Finally, tonemap the color.
    outColor.rgb = tonemap(outColor.rgb);

    // Debugging
    // Shader inputs debug visualization
    if (sceneData.params.z > 0.0) {
        int index = int(sceneData.params.z);
        switch (index) {
            // Base Color Texture
            case 1:
                outColor.rgba = baseColor;
                break;
            // Normal
            case 2:
                vec3 n = getNormal();
                outColor.rgb = n * 0.5 + 0.5;
                break;
            // Occlusion
            case 3:
                outColor.rgb = ((materialFlags & TEXTURE_FLAG_HAS_AO_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[baseTextureID + 1], inUV).rrr;
                break;
            // Emission
            case 4:
                outColor.rgb = ((materialFlags & TEXTURE_FLAG_HAS_EMISSION_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[baseTextureID + 3], inUV).rgb;
                break;
            // Roughness
            case 5:
                outColor.rgb = ((materialFlags & HAS_METALLIC_ROUGHNESS_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[baseTextureID + 1], inUV).ggg;
                break;
            // Metallic
            case 6:
                outColor.rgb = ((materialFlags & HAS_METALLIC_ROUGHNESS_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[baseTextureID + 1], inUV).bbb;
                break;
        }
        outColor = outColor;
    }
}
