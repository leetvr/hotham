#version 460
#extension GL_GOOGLE_include_directive : require
#include "../../../../hotham/src/shaders/common.glsl"
#include "../../../../hotham/src/shaders/lights.glsl"
#include "../../../../hotham/src/shaders/brdf.glsl"
#include "../../../../hotham/src/shaders/pbr.glsl"

// Inputs
layout (location = 0) in vec4 inRayOrigin;
layout (location = 1) in flat uint inInstanceIndex;

#include "quadric.glsl"

// Outputs
layout (location = 0) out vec4 outColor;
layout (depth_less) out float gl_FragDepth;

// These values are from https://registry.khronos.org/vulkan/specs/1.2-extensions/html/chap27.html#primrast-samplelocations
const vec2 offsetSample0 = vec2(0.375 - 0.5, 0.125 - 0.5);
const vec2 offsetSample1 = vec2(0.875 - 0.5, 0.375 - 0.5);
const vec2 offsetSample2 = vec2(0.125 - 0.5, 0.625 - 0.5);
const vec2 offsetSample3 = vec2(0.625 - 0.5, 0.875 - 0.5);

void main() {
    // Start by setting the output color to a familiar "error" magenta.
    outColor = ERROR_MAGENTA;

    // Retrieve draw data
    QuadricData d = quadricDataBuffer.data[inInstanceIndex];

    // Find ray-quadric intersection, if any
    vec4 rayOrigin = inRayOrigin / inRayOrigin.w;
    vec4 rayDir = vec4(normalize(rayOrigin.xyz - sceneData.cameraPosition[gl_ViewIndex].xyz), 0.0);

    vec4 surfaceQTimesRayOrigin = d.surfaceQ * rayOrigin;
    vec4 surfaceQTimesRayDir = d.surfaceQ * rayDir;

    float a = dot(rayDir, surfaceQTimesRayDir);
    float b = dot(rayOrigin, surfaceQTimesRayDir);
    float c = dot(rayOrigin, surfaceQTimesRayOrigin);
    // Discriminant from quadratic formula is
    // b^2 - 4ac
    // but we are able to simplify it by substituting b with b/2.
    float discriminant = b * b - a * c;
    vec2 gradientOfDiscriminant = vec2(dFdx(discriminant), dFdy(discriminant));
    gl_SampleMask[0] = int(
        step(0.0, discriminant + dot(offsetSample0, gradientOfDiscriminant)) +
        step(0.0, discriminant + dot(offsetSample1, gradientOfDiscriminant)) * 2 +
        step(0.0, discriminant + dot(offsetSample2, gradientOfDiscriminant)) * 4 +
        step(0.0, discriminant + dot(offsetSample3, gradientOfDiscriminant)) * 8);

    // Pick the solution that is facing us
    float t = -(b + sqrt(max(0.0, discriminant))) / a;

    if (t < -0.0001) {
        t = 0.0;
        gl_SampleMask[0] = 0;
    }

    // hitPoint.w = 1 because rayOrigin.w = 1 and rayDir.w = 0.
    vec4 hitPoint = rayOrigin + rayDir * t;
    float boundsValue = 0.0001 - dot(hitPoint, d.boundsQ * hitPoint);
    vec2 gradientOfBoundsValue = vec2(dFdx(boundsValue), dFdy(boundsValue));
    gl_SampleMask[0] &= int(
        step(0.0, boundsValue + dot(offsetSample0, gradientOfBoundsValue)) +
        step(0.0, boundsValue + dot(offsetSample1, gradientOfBoundsValue)) * 2 +
        step(0.0, boundsValue + dot(offsetSample2, gradientOfBoundsValue)) * 4 +
        step(0.0, boundsValue + dot(offsetSample3, gradientOfBoundsValue)) * 8);

    // Discarding is postponed until here to make sure the derivatives above are valid.
    if (gl_SampleMask[0] == 0) {
        discard;
    }

    // Compute depth
    vec4 v_clip_coord = sceneData.viewProjection[gl_ViewIndex] * hitPoint;
    gl_FragDepth = v_clip_coord.z / v_clip_coord.w;

    // Set globals that are read inside functions for lighting etc.
    p = hitPoint.xyz;
    v = normalize(sceneData.cameraPosition[gl_ViewIndex].xyz - p);

    // Compute normal from gradient of surface quadric
    n = normalize((d.surfaceQ * hitPoint).xyz);

    vec4 uv4 = d.uvFromGos * hitPoint;
    uv = uv4.xy / uv4.w;

    // Unpack the material parameters
    materialFlags = material.flagsAndBaseTextureID & 0xFFFF;
    baseTextureID = material.flagsAndBaseTextureID >> 16;
    metallicRoughnessAlphaMaskCutoff = unpackUnorm4x8(
        material.packedMetallicRoughnessFactorAlphaMaskCutoff).xyz;

    // Determine the base color
    vec4 baseColor = unpackUnorm4x8(material.packedBaseColor);

    if ((materialFlags & HAS_BASE_COLOR_TEXTURE) != 0) {
        baseColor *= texture(textures[baseTextureID], uv);
    }

    // Handle transparency
    if (metallicRoughnessAlphaMaskCutoff.z > 0.0f) {
        if (baseColor.a < metallicRoughnessAlphaMaskCutoff.z) {
            // TODO: Apparently Adreno GPUs don't like discarding.
            discard;
        }
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
                outColor.rgb = n * 0.5 + 0.5;
                break;
            // Occlusion
            case 3:
                outColor.rgb = ((materialFlags & MATERIAL_FLAG_HAS_AO_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[baseTextureID + 1], uv).rrr;
                break;
            // Emission
            case 4:
                outColor.rgb = ((materialFlags & MATERIAL_FLAG_HAS_EMISSION_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[baseTextureID + 3], uv).rgb;
                break;
            // Roughness
            case 5:
                outColor.rgb = ((materialFlags & HAS_METALLIC_ROUGHNESS_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[baseTextureID + 1], uv).ggg;
                break;
            // Metallic
            case 6:
                outColor.rgb = ((materialFlags & HAS_METALLIC_ROUGHNESS_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[baseTextureID + 1], uv).bbb;
                break;
        }
        outColor = outColor;
    }
}
