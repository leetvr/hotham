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

    // Determine the base color
    vec4 baseColor;

    if ((material.textureFlags & TEXTURE_FLAG_HAS_PBR_TEXTURES) == 0) {
        baseColor = vec4(1.0, 1.0, 1.0, 1.0);
    } else {
        baseColor = texture(textures[material.baseTextureID], inUV);
    }

    outColor.rgb = getPBRMetallicRoughnessColor(baseColor);

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
                outColor.rgb = ((material.textureFlags & TEXTURE_FLAG_HAS_AO_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[material.baseTextureID + 1], inUV).rrr;
                break;
            // Emission
            case 4:
                outColor.rgb = ((material.textureFlags & TEXTURE_FLAG_HAS_EMISSION_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[material.baseTextureID + 3], inUV).rgb;
                break;
            // Roughness
            case 5:
                outColor.rgb = ((material.textureFlags & TEXTURE_FLAG_HAS_PBR_TEXTURES) != 0) ? ERROR_MAGENTA.rgb : texture(textures[material.baseTextureID + 1], inUV).ggg;
                break;
            // Metallic
            case 6:
                outColor.rgb = ((material.textureFlags & TEXTURE_FLAG_HAS_PBR_TEXTURES) != 0) ? ERROR_MAGENTA.rgb : texture(textures[material.baseTextureID + 1], inUV).bbb;
                break;
        }
        outColor = outColor;
    }
}
