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

// Based on https://iquilezles.org/articles/filterableprocedurals/
float filteredGrid( in vec2 p, in vec2 dpdx, in vec2 dpdy )
{
    const float N = 10.0;
    vec2 w = max(abs(dpdx), abs(dpdy)) + 0.001;
    vec2 a = p + 0.5*w;
    vec2 b = p - 0.5*w;
    vec2 i = (floor(a)+min(fract(a)*N,1.0)-
              floor(b)-min(fract(b)*N,1.0))/(N*w);
    return (1.0-i.x)*(1.0-i.y);
}

void main() {
    // Start by setting the output color to a familiar "error" magenta.
    outColor = ERROR_MAGENTA;

    // Retrieve draw data
    QuadricData d = quadricDataBuffer.data[inInstanceIndex];

    // Find ray-quadric intersection, if any
    vec4 rayOrigin = sceneData.cameraPosition[gl_ViewIndex];
    vec4 rayDir = vec4(inRayOrigin.xyz / inRayOrigin.w - sceneData.cameraPosition[gl_ViewIndex].xyz, 0.0);

    // A point p on the ray is
    // p = rayOrigin + rayDir*t
    // The point is on the surface of the quadric Q when
    // pᵀ*Q*p = 0
    // These can be combined and manipulated to give a quadratic equation in standard form:
    // (rayOrigin + rayDir*t)ᵀ*Q*(rayOrigin + rayDir*t) = 0
    // (rayOriginᵀ + rayDirᵀ*t)*(Q*rayOrigin + Q*rayDir*t) = 0
    // (rayOriginᵀ*(Q*rayOrigin + Q*rayDir*t) + rayDirᵀ*t*(Q*rayOrigin + Q*rayDir*t) = 0
    // rayOriginᵀ*Q*rayOrigin + rayOriginᵀ*Q*rayDir*t + rayDirᵀ*t*Q*rayOrigin + rayDirᵀ*t*Q*rayDir*t = 0
    // rayOriginᵀ*Q*rayOrigin + 2*rayOriginᵀ*Q*rayDir*t + rayDirᵀ*Q*rayDir*t*t = 0

    // The quadratic formula based on ax² + bx + c = 0 has the solutions
    // x = (-b ± √(b² - 4ac)) / 2a
    // but we can simplify it by basing it on ax² + 2bx + c = 0.
    // x = (-2b ± √(4b² - 4ac)) / 2a
    // x = (-2b ± 2√(b² - ac)) / 2a
    // x = (-b ± √(b² - ac)) / a
    // We are only interested in the solution for when the surface is facing us.
    // The ray direction and surface normal should be more than 90 degrees apart
    // rayDir ⬤ n < 0
    // The surface normal is proportional to the gradient of the quadric.
    // n ~= Q₃*p
    // rayDirᵀ*Q₃*p < 0
    // rayDirᵀ*Q₃*(rayOrigin + rayDir*t) < 0
    // rayDirᵀ*Q₃*rayOrigin + rayDirᵀ*Q₃*rayDir*t < 0
    // b + a*t < 0
    // a*t < -b
    // This allows us to pick a single solution ("±" becomes "-")
    // x = (-b ± √(b² - ac)) / a
    // a*x = -b ± √(b² - ac)
    // a*t = -b - √(b² - ac),  because a*t < -b and √(b² - ac) > 0
    // t = (-b - √(b² - ac)) / a
    // What if a = 0?
    // The "Citardauq Formula" works even when a = 0.
    // It can be derived by expanding the fraction with the "conjugate" of the numerator.
    // t = (-b - √(b² - ac)) / a * (-b + √(b² - ac)) / (-b + √(b² - ac))
    // t = c / (-b + √(b² - ac))
    // This is better but can still get division by zero if ac = 0 and b > 0.
    // We will also have catastrophic cancellation if b > 0 and |ac| << b²
    // b = rayOriginᵀ*Q*rayDir

    vec4 surfaceQTimesRayOrigin = d.surfaceQ * rayOrigin;
    vec4 surfaceQTimesRayDir = d.surfaceQ * rayDir;

    float a = dot(rayDir, surfaceQTimesRayDir);
    float b = dot(rayDir, surfaceQTimesRayOrigin);
    float c = dot(rayOrigin, surfaceQTimesRayOrigin);

    float discriminant = b * b - a * c;
    vec2 gradientOfDiscriminant = vec2(dFdx(discriminant), dFdy(discriminant));
    gl_SampleMask[0] = int(
        step(0.0, discriminant + dot(offsetSample0, gradientOfDiscriminant)) +
        step(0.0, discriminant + dot(offsetSample1, gradientOfDiscriminant)) * 2 +
        step(0.0, discriminant + dot(offsetSample2, gradientOfDiscriminant)) * 4 +
        step(0.0, discriminant + dot(offsetSample3, gradientOfDiscriminant)) * 8);

    // Pick the solution that is facing us
    float t = c / (-b + sqrt(max(0.0, discriminant)));
    // hitPoint.w = 1 because rayOrigin.w = 1 and rayDir.w = 0.
    vec4 hitPoint = rayOrigin + rayDir * t;

    // Compute gradient along the surface (orthogonal to surface normal).
    // Clamp them to avoid flying pixels due to overshooting.
    vec3 ddx_hitPoint = clamp(dFdx(hitPoint.xyz), -1.0, 1.0);
    vec3 ddy_hitPoint = clamp(dFdy(hitPoint.xyz), -1.0, 1.0);

    // Discarding is postponed until here to make sure the derivatives above are valid.
    if (t < 1.0) {
        discard;
    }

    vec4 boundsQTimesHitPoint = d.boundsQ * hitPoint;
    float boundsValue = dot(hitPoint, boundsQTimesHitPoint);
    vec2 gradientOfBoundsValue = vec2(
        dot(ddx_hitPoint, boundsQTimesHitPoint.xyz),
        dot(ddy_hitPoint, boundsQTimesHitPoint.xyz));
    gl_SampleMask[0] &= int(
        step(boundsValue + dot(offsetSample0, gradientOfBoundsValue), 0.0) +
        step(boundsValue + dot(offsetSample1, gradientOfBoundsValue), 0.0) * 2 +
        step(boundsValue + dot(offsetSample2, gradientOfBoundsValue), 0.0) * 4 +
        step(boundsValue + dot(offsetSample3, gradientOfBoundsValue), 0.0) * 8);

    // Discard if all samples have been masked out.
    if (gl_SampleMask[0] == 0) {
        discard;
    }

    // Compute depth
    vec4 v_clip_coord = sceneData.viewProjection[gl_ViewIndex] * hitPoint;
    gl_FragDepth = v_clip_coord.z / v_clip_coord.w;

    // Set globals that are read inside functions for lighting etc.
    pos = hitPoint.xyz;
    v = normalize(sceneData.cameraPosition[gl_ViewIndex].xyz - pos);

    // Compute normal from gradient of surface quadric.
    n = normalize((d.surfaceQ * hitPoint).xyz);

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
    baseColor.rgb *= V16(filteredGrid(uv, ddx_uv, ddy_uv));

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
