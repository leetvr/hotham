#version 460

#extension GL_GOOGLE_include_directive : require
#extension GL_EXT_shader_explicit_arithmetic_types_int16 : require
#extension GL_EXT_shader_explicit_arithmetic_types_float16 : require
#extension GL_EXT_shader_16bit_storage : require

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

const float N = 10.0; // grid ratio
float gridTextureGradBox( in vec2 p, in vec2 ddx, in vec2 ddy )
{
	// filter kernel
    vec2 w = max(abs(ddx), abs(ddy)) + 0.01;

	// analytic (box) filtering
    vec2 a = p + 0.5*w;
    vec2 b = p - 0.5*w;
    vec2 i = (floor(a)+min(fract(a)*N,1.0)-
              floor(b)-min(fract(b)*N,1.0))/(N*w);
    //pattern
    return (1.0-i.x)*(1.0-i.y);
}

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
    // float t = -(b + sqrt(max(0.0, discriminant))) / a;
    // The "Citardauq Formula" works even if a is zero.
    float t = c / -(b + sqrt(max(0.0, discriminant)));

    if (t < -0.0001) {
        t = 0.0;
        gl_SampleMask[0] = 0;
    }

    // hitPoint.w = 1 because rayOrigin.w = 1 and rayDir.w = 0.
    vec4 hitPoint = rayOrigin + rayDir * t;
    // Compute normal from gradient of surface quadric.
    n = normalize((d.surfaceQ * hitPoint).xyz);
    // Compute gradient along the surface (orthogonal to surface normal).
    vec3 ddx_hitPoint = dFdx(rayOrigin.xyz) + dFdx(rayDir.xyz) * t;
    vec3 ddy_hitPoint = dFdy(rayOrigin.xyz) + dFdy(rayDir.xyz) * t;
    ddx_hitPoint -= rayDir.xyz * (dot(ddx_hitPoint, n) / dot(rayDir.xyz, n));
    ddy_hitPoint -= rayDir.xyz * (dot(ddy_hitPoint, n) / dot(rayDir.xyz, n));

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
    pos = hitPoint.xyz;
    v = normalize(sceneData.cameraPosition[gl_ViewIndex].xyz - pos);

    vec4 uv4 = d.uvFromGos * hitPoint;
    uv = uv4.xy / uv4.w;
    mat3x2 uvFromGos23 = mat3x2(d.uvFromGos[0].xy, d.uvFromGos[1].xy, d.uvFromGos[2].xy);
    vec2 ddx_uv = uvFromGos23 * ddx_hitPoint / uv4.w; // TODO: Handle derivative of w.
    vec2 ddy_uv = uvFromGos23 * ddy_hitPoint / uv4.w;

    // Unpack the material parameters
    materialFlags = material.flagsAndBaseTextureID & 0xFFFF;
    baseTextureID = material.flagsAndBaseTextureID >> 16;

    // Determine the base color
    f16vec3 baseColor = V16(unpackUnorm4x8(material.packedBaseColor));
    baseColor.rgb *= V16(gridTextureGradBox(uv, ddx_uv, ddy_uv));

    if ((materialFlags & MATERIAL_FLAG_HAS_BASE_COLOR_TEXTURE) != 0) {
        baseColor *= V16(texture(textures[baseTextureID], uv));
    }

    // Choose the correct workflow for this material
    if ((materialFlags & PBR_WORKFLOW_UNLIT) == 0) {
        outColor.rgb = getPBRMetallicRoughnessColor(baseColor);
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
                outColor.rgb = ((materialFlags & MATERIAL_FLAG_HAS_AO_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[baseTextureID + 1], uv).rrr;
                break;
            // Emission
            case 4:
                outColor.rgb = ((materialFlags & MATERIAL_FLAG_HAS_EMISSION_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[baseTextureID + 3], uv).rgb;
                break;
            // Roughness
            case 5:
                outColor.rgb = ((materialFlags & MATERIAL_FLAG_HAS_METALLIC_ROUGHNESS_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[baseTextureID + 1], uv).ggg;
                break;
            // Metallic
            case 6:
                outColor.rgb = ((materialFlags & MATERIAL_FLAG_HAS_METALLIC_ROUGHNESS_TEXTURE) != 0) ? ERROR_MAGENTA.rgb : texture(textures[baseTextureID + 1], uv).bbb;
                break;
        }
        outColor = outColor;
    }
}
