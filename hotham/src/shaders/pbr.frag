// PBR shader based on the Khronos glTF-Sample Viewer:
// https://github.com/KhronosGroup/glTF-WebGL-PBR
#version 460

#extension GL_GOOGLE_include_directive : require
#extension GL_EXT_shader_explicit_arithmetic_types_int16 : require
#extension GL_EXT_shader_explicit_arithmetic_types_float16 : require
#extension GL_EXT_shader_16bit_storage : require

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

// Get normal, tangent and bitangent vectors.
vec3 getNormal() {
    vec3 N = normalize(inNormal);

    // If we don't have a normal texture, then just use the vertex normal
    if ((materialFlags & MATERIAL_FLAG_HAS_NORMAL_TEXTURE) == 0) {
        return N;
    }

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
    // Unpack the material parameters
    materialFlags = material.flagsAndBaseTextureID & 0xFFFF;
    baseTextureID = material.flagsAndBaseTextureID >> 16;

    // Determine the base color
    f16vec3 baseColor;

    if ((materialFlags & MATERIAL_FLAG_HAS_BASE_COLOR_TEXTURE) != 0) {
        // This is *technically* against the spec, since material base color is meant to be treated as a "factor",
        // but as of writing no texture authoring tool actually changes these values, so we can skip unnecessary
        // arithmetic.
        baseColor = V16(texture(textures[baseTextureID], inUV));
    } else {
        // If no base color texture is present, check to see if the material had the base color factors set. This
        // is usually only for very simple materials or prototyping.`
        baseColor = V16(unpackUnorm4x8(material.packedBaseColor));
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
                outColor.rgb = ((materialFlags & MATERIAL_FLAG_HAS_AO_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[baseTextureID + 1], inUV).rrr;
                break;
            // Emission
            case 4:
                outColor.rgb = ((materialFlags & MATERIAL_FLAG_HAS_EMISSION_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[baseTextureID + 3], inUV).rgb;
                break;
            // Roughness
            case 5:
                outColor.rgb = ((materialFlags & MATERIAL_FLAG_HAS_METALLIC_ROUGHNESS_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[baseTextureID + 1], inUV).ggg;
                break;
            // Metallic
            case 6:
                outColor.rgb = ((materialFlags & MATERIAL_FLAG_HAS_METALLIC_ROUGHNESS_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[baseTextureID + 1], inUV).bbb;
                break;
        }
        outColor = outColor;
    }
}
