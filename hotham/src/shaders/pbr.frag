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
    f16vec4 color = ERROR_MAGENTA;

    // Retrieve the material from the buffer.
    Material material = materialBuffer.materials[inMaterialID];

    // Determine the base color
    f16vec4 baseColor;

    if (material.baseColorTextureID == NOT_PRESENT) {
        baseColor = f16vec4(material.baseColorFactor);
    } else {
        baseColor = f16vec4(texture(textures[material.baseColorTextureID], inUV)) * f16vec4(material.baseColorFactor);
    }

    // Handle transparency
    if (material.alphaMask == 1.0f) {
        if (baseColor.a < material.alphaMaskCutoff) {
            // TODO: Apparently Adreno GPUs don't like discarding.
            discard;
        }
    }

    // Choose the correct workflow for this material
    if (material.workflow == PBR_WORKFLOW_METALLIC_ROUGHNESS) {
        color.rgb = getPBRMetallicRoughnessColor(material, baseColor);
    } else if (material.workflow == PBR_WORKFLOW_UNLIT) {
        color = baseColor;
    }

    // Finally, tonemap the color.
    outColor.rgb = vec3(tonemap(color.rgb));

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
                outColor.rgb = vec3(getNormal(material.normalTextureID)) * 0.5 + 0.5;
                break;
            // Occlusion
            case 3:
                outColor.rgb = (material.occlusionTextureID == NOT_PRESENT) ? ERROR_MAGENTA.rgb : texture(textures[material.occlusionTextureID], inUV).rrr;
                break;
            // Emission
            case 4:
                outColor.rgb = (material.emissiveTextureID == NOT_PRESENT) ? ERROR_MAGENTA.rgb : texture(textures[material.emissiveTextureID], inUV).rgb;
                break;
            // Roughness
            case 5:
                outColor.rgb = (material.metallicRoughnessTextureID == NOT_PRESENT) ? ERROR_MAGENTA.rgb : texture(textures[material.metallicRoughnessTextureID], inUV).ggg;
                break;
            // Metallic
            case 6:
                outColor.rgb = (material.metallicRoughnessTextureID == NOT_PRESENT) ? ERROR_MAGENTA.rgb : texture(textures[material.metallicRoughnessTextureID], inUV).bbb;
                break;
        }
    }
}
