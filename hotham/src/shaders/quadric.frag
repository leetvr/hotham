#version 460
#include "common.glsl"
#include "lights.glsl"
#include "brdf.glsl"

// Textures
layout (set = 0, binding = 4) uniform sampler2D textures[];
layout (set = 0, binding = 5) uniform samplerCube cubeTextures[];

#include "pbr.glsl"

// Inputs
layout (location = 0) in vec4 inRayOrigin;
layout (location = 1) in vec4 inRayDir;
layout (location = 2) in vec4 inSurfaceQTimesRayOrigin;
layout (location = 3) in vec4 inSurfaceQTimesRayDir;
layout (location = 4) in flat uint inInstanceIndex;

layout (std430, set = 0, binding = 6) readonly buffer QuadricDataBuffer {
    QuadricData data[];
} quadricDataBuffer;

layout (std430, set = 0, binding = 1) readonly buffer MaterialBuffer {
    Material materials[];
} materialBuffer;

// Outputs
layout (location = 0) out vec4 outColor;
layout (depth_less) out float gl_FragDepth;

void main() {
    // Start by setting the output color to a familiar "error" magenta.
    outColor = ERROR_MAGENTA;

    // Retrieve draw data
    QuadricData d = quadricDataBuffer.data[inInstanceIndex];

    float coverage = 0.0;
    vec4 hitPoint = vec4(0.0, 0.0, 0.0, 0.0);

    for (int sampleId = 0; sampleId < 4; ++sampleId) {
        vec4 sampleRayOrigin = interpolateAtSample(inRayOrigin, sampleId);
        vec4 sampleRayDir = interpolateAtSample(inRayDir, sampleId);
        vec4 sampleSurfaceQTimesRayOrigin = interpolateAtSample(inSurfaceQTimesRayOrigin, sampleId);
        vec4 sampleSurfaceQTimesRayDir = interpolateAtSample(inSurfaceQTimesRayDir, sampleId);

        // Find ray-quadric intersection, if any
        float a = dot(sampleRayDir, sampleSurfaceQTimesRayDir);
        float b = dot(sampleRayOrigin, sampleSurfaceQTimesRayDir) + dot(sampleRayDir, sampleSurfaceQTimesRayOrigin);
        float c = dot(sampleRayOrigin, sampleSurfaceQTimesRayOrigin);
        // Discriminant from quadratic formula
        // b^2 - 4ac
        float discriminant = b * b - 4.0 * a * c;
        if (discriminant < 0.0) {
            continue;
        }

        // Pick the solution that is facing us
        float t = (b + sqrt(discriminant)) * -0.5 / a;

        if (t < 0.0) {
            continue;
        }

        vec4 sampleHitPoint = sampleRayOrigin + sampleRayDir * t.x;
        float boundsValue = dot(sampleHitPoint, d.boundsQ * sampleHitPoint);
        if (boundsValue <= 0.0) {
            coverage += 0.25;
            hitPoint += sampleHitPoint;
        }
    }

    // Discarding is postponed until all samples have been tested.
    if (coverage == 0.0) {
        discard;
    }

    // We divide with w here because hitPoint is used without w further down.
    hitPoint /= hitPoint.w;

    // Compute depth
    vec4 v_clip_coord = sceneData.viewProjection[gl_ViewIndex] * hitPoint;
    float f_ndc_depth = v_clip_coord.z / v_clip_coord.w;
    gl_FragDepth = f_ndc_depth;

    // Compute normal from gradient of surface quadric
    vec3 normal = normalize((d.surfaceQ * hitPoint).xyz);

    vec4 uv4 = d.uvFromGlobal * hitPoint;
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
    outColor.a = coverage;

    // Debugging
    // Shader inputs debug visualization
    // "none", "Base Color Texture", "Normal Texture", "Occlusion Texture", "Emissive Texture", "Metallic (?)", "Roughness (?)"
    if (sceneData.params.z > 0.0) {
        int index = int(sceneData.params.z);
        switch (index) {
            case 1:
                outColor.rgba = baseColor;
                break;
            case 2:
                outColor.rgb = normal * 0.5 + 0.5;
                break;
            case 3:
                outColor.rgb = (material.occlusionTextureID == NOT_PRESENT) ? ERROR_MAGENTA.rgb : texture(textures[material.occlusionTextureID], uv).ggg;
                break;
            case 4:
                outColor.rgb = (material.emissiveTextureID == NOT_PRESENT) ? ERROR_MAGENTA.rgb : texture(textures[material.emissiveTextureID], uv).rgb;
                break;
            case 5:
                outColor.rgb = (material.physicalDescriptorTextureID == NOT_PRESENT) ? ERROR_MAGENTA.rgb : texture(textures[material.physicalDescriptorTextureID], uv).ggg;
                break;
            case 6:
                outColor.rgb = (material.physicalDescriptorTextureID == NOT_PRESENT) ? ERROR_MAGENTA.rgb : texture(textures[material.physicalDescriptorTextureID], uv).aaa;
                break;
        }
        outColor = outColor;
    }
}
