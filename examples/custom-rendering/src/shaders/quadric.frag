#version 460
#extension GL_GOOGLE_include_directive : require
#include "../../../../hotham/src/shaders/common.glsl"
#include "../../../../hotham/src/shaders/lights.glsl"
#include "../../../../hotham/src/shaders/brdf.glsl"

// Textures
layout (set = 0, binding = 4) uniform sampler2D textures[];
layout (set = 0, binding = 5) uniform samplerCube cubeTextures[];

#include "pbr.glsl"

// Inputs
layout (location = 0) in vec4 inRayOrigin;
layout (location = 1) in flat uint inInstanceIndex;

#include "quadric.glsl"

layout (std430, set = 0, binding = 1) readonly buffer MaterialBuffer {
    Material materials[];
} materialBuffer;

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

    // Compute normal from gradient of surface quadric
    vec3 normal = normalize((d.surfaceQ * hitPoint).xyz);

    vec4 uv4 = d.uvFromGos * hitPoint;
    vec2 uv = uv4.xy / uv4.w;

    // Retrieve the material from the buffer.
    Material material = materialBuffer.materials[d.materialID];

    // Determine the base color
    vec4 baseColor;

    if (material.baseColorTextureID == NOT_PRESENT) {
        baseColor = material.baseColorFactor;
    } else {
        baseColor = texture(textures[material.baseColorTextureID], uv) * material.baseColorFactor;
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
        outColor.rgb = getPBRMetallicRoughnessColor(material, baseColor, hitPoint.xyz, normal, uv);
    } else if (material.workflow == PBR_WORKFLOW_UNLIT) {
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
                outColor.rgb = normal * 0.5 + 0.5;
                break;
            // Occlusion
            case 3:
                outColor.rgb = (material.occlusionTextureID == NOT_PRESENT) ? ERROR_MAGENTA.rgb : texture(textures[material.occlusionTextureID], uv).rrr;
                break;
            // Emission
            case 4:
                outColor.rgb = (material.emissiveTextureID == NOT_PRESENT) ? ERROR_MAGENTA.rgb : texture(textures[material.emissiveTextureID], uv).rgb;
                break;
            // Roughness
            case 5:
                outColor.rgb = (material.metallicRoughnessTextureID == NOT_PRESENT) ? ERROR_MAGENTA.rgb : texture(textures[material.metallicRoughnessTextureID], uv).ggg;
                break;
            // Metallic
            case 6:
                outColor.rgb = (material.metallicRoughnessTextureID == NOT_PRESENT) ? ERROR_MAGENTA.rgb : texture(textures[material.metallicRoughnessTextureID], uv).bbb;
                break;
        }
        outColor = outColor;
    }
}
