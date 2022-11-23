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
layout (location = 2) in vec3 inNormal;

// Textures
layout (set = 0, binding = 3) uniform sampler2D textures[];
layout (set = 0, binding = 4) uniform samplerCube cubeTextures[];

#include "pbr.glsl"

// Outputs
layout (location = 0) out vec4 outColor;

// Get normal, tangent and bitangent vectors.
vec3 getNormal() {
    vec3 N = normalize(inNormal);
    if ((materialFlags & TEXTURE_FLAG_HAS_NORMAL_MAP) == 0) {
        return N;
    }

    vec3 textureNormal;
    textureNormal.xy = texture(textures[baseTextureID + 2], inUV).ga * 2.0 - 1.0;
    textureNormal.z = sqrt(1 - dot(textureNormal.xy, textureNormal.xy));

    // We compute the tangents on the fly because it is faster, presumably because it saves bandwidth.
    // See http://www.thetenthplanet.de/archives/1180 for an explanation of how this works
    // and a little bit about why it is better than using precomputed tangents.
    // Note however that we are using a slightly different formulation with coordinates in
    // globally oriented stage space instead of view space and we rely on the UV map not being too distorted.
    vec3 dGosPosDx = dFdx(inGosPos);
    vec3 dGosPosDy = dFdy(inGosPos);
    vec2 dUvDx = dFdx(inUV);
    vec2 dUvDy = dFdy(inUV);

    vec3 T = normalize(dGosPosDx * dUvDy.t - dGosPosDy * dUvDx.t);
    vec3 B = normalize(cross(N, T));
    mat3 TBN = mat3(T, B, N);

    return normalize(TBN * textureNormal);
}

void main() {
    // Start by setting the output color to a familiar "error" magenta.
    outColor = ERROR_MAGENTA;

    // Unpack the material parameters
    materialFlags = material.flagsAndBaseTextureID & 0xFFFF;
    baseTextureID = material.flagsAndBaseTextureID >> 16;
    metallicRoughnessAlphaMaskCutoff = unpackUnorm4x8(
        material.packedMetallicRoughnessFactorAlphaMaskCutoff).xyz;

    // Determine the base color
    vec4 baseColor = unpackUnorm4x8(material.packedBaseColor);

    if ((materialFlags & HAS_BASE_COLOR_TEXTURE) != 0) {
        baseColor *= texture(textures[baseTextureID], inUV);
    }

    // Handle transparency
    if (metallicRoughnessAlphaMaskCutoff.z > 0.0f) {
        if (baseColor.a < metallicRoughnessAlphaMaskCutoff.z) {
            // TODO: Apparently Adreno GPUs don't like discarding.
            discard;
        }
    }

    // Set globals that are read inside functions for lighting etc.
    p = inGosPos;
    v = normalize(sceneData.cameraPosition[gl_ViewIndex].xyz - inGosPos);
    n = getNormal();
    uv = inUV;

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
