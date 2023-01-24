// PBR shader based on the Khronos glTF-Sample Viewer:
// https://github.com/KhronosGroup/glTF-WebGL-PBR

#version 460

#extension GL_GOOGLE_include_directive : require
#extension GL_EXT_shader_explicit_arithmetic_types_int16 : require
#extension GL_EXT_shader_explicit_arithmetic_types_float16 : require
#extension GL_EXT_shader_16bit_storage : require
#extension GL_EXT_multiview : enable

#include "common.glsl"
#include "lights.glsl"
#include "brdf.glsl"
#include "pbr.glsl"

// Inputs
layout (location = 0) in vec3 inGosPos;
layout (location = 1) in vec2 inUV;
layout (location = 2) in vec3 inNormal;

// Outputs
layout (location = 0) out vec4 outColor;

// Fast approximation of ACES tonemap
// https://knarkowicz.wordpress.com/2016/01/06/aces-filmic-tone-mapping-curve/
f16vec3 toneMapACES_Narkowicz(f16vec3 color) {
    const float16_t A = F16(2.51);
    const float16_t B = F16(0.03);
    const float16_t C = F16(2.43);
    const float16_t D = F16(0.59);
    const float16_t E = F16(0.14);
    return clamp((color * (A * color + B)) / (color * (C * color + D) + E), F16(0), F16(1));
}

f16vec3 tonemap(const f16vec3 color) {
    return toneMapACES_Narkowicz(color);
}

// Get normal, tangent and bitangent vectors.
vec3 getNormal() {
    vec3 N = normalize(inNormal);

    f16vec3 textureNormal;
    textureNormal.xy = f16vec2(texture(textures[baseTextureID + 2], inUV).ga) * F16(2) - F16(1);
    textureNormal.z = sqrt(F16(1) - dot(textureNormal.xy, textureNormal.xy));

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
    f16vec3 baseColor = V16(unpackUnorm4x8(material.packedBaseColor));

    if ((materialFlags & TEXTURE_FLAG_HAS_BASE_COLOR_TEXTURE) != 0) {
        baseColor *= V16(texture(textures[baseTextureID], inUV));
    }

    // Set globals that are read inside functions for lighting etc.
    pos = inGosPos;
    v = normalize(sceneData.cameraPosition[gl_ViewIndex].xyz - inGosPos);
    n = getNormal();
    uv = inUV;

    // Choose the correct workflow for this material
    if ((materialFlags & PBR_WORKFLOW_UNLIT) == 0) {
        outColor.rgb = tonemap(getPBRMetallicRoughnessColor(baseColor));
    } else {
        outColor.rgb = tonemap(baseColor);
    }

    // Debugging
    // Shader inputs debug visualization
    if (sceneData.params.z > 0.0) {
        int index = int(sceneData.params.z);
        switch (index) {
            // Base Color Texture
            case 1:
                outColor.rgb = baseColor;
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
                outColor.rgb = ((materialFlags & TEXTURE_FLAG_HAS_METALLIC_ROUGHNESS_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[baseTextureID + 1], inUV).ggg;
                break;
            // Metallic
            case 6:
                outColor.rgb = ((materialFlags & TEXTURE_FLAG_HAS_METALLIC_ROUGHNESS_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[baseTextureID + 1], inUV).bbb;
                break;
        }
        outColor = outColor;
    }
}
