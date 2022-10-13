// PBR shader based on the Khronos glTF-Sample Viewer:
// https://github.com/KhronosGroup/glTF-WebGL-PBR

#version 460
#include "common.glsl"
#include "lights.glsl"
#include "brdf.glsl"

// Textures
layout (set = 0, binding = 4) uniform sampler2D textures[];
layout (set = 0, binding = 5) uniform samplerCube cubeTextures[];

#include "pbr.glsl"

// Inputs
layout (location = 0) in vec3 inGosPos;
layout (location = 1) in vec2 inUV;
layout (location = 2) flat in uint inMaterialID;
layout (location = 3) in vec3 inNormal;

layout (std430, set = 0, binding = 0) readonly buffer DrawDataBuffer {
    DrawData data[];
} drawDataBuffer;

layout (std430, set = 0, binding = 1) readonly buffer MaterialBuffer {
    Material materials[];
} materialBuffer;

// Outputs
layout (location = 0) out vec4 outColor;

// Get normal, tangent and bitangent vectors.
vec3 getNormal(uint normalTextureID) {
    vec3 N = normalize(inNormal);
    if (normalTextureID == NOT_PRESENT) {
        return N;
    }

    vec3 textureNormal;
    textureNormal.xy = texture(textures[normalTextureID], inUV).ga * 2.0 - 1.0;
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

    // Retrieve the material from the buffer.
    Material material = materialBuffer.materials[inMaterialID];

    // Determine the base color
    vec4 baseColor;

    if (material.baseColorTextureID == NOT_PRESENT) {
        baseColor = material.baseColorFactor;
    } else {
        baseColor = texture(textures[material.baseColorTextureID], inUV) * material.baseColorFactor;
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
        // Get the normal
        vec3 n = getNormal(material.normalTextureID);
        outColor.rgb = getPBRMetallicRoughnessColor(material, baseColor, inGosPos, n, inUV);
    } else if (material.workflow == PBR_WORKFLOW_UNLIT) {
        outColor = baseColor;
    }

    // Finally, tonemap the color.
    outColor.rgb = tonemap(outColor.rgb);

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
                vec3 n = getNormal(material.normalTextureID);
                outColor.rgb = n * 0.5 + 0.5;
                break;
            case 3:
                outColor.rgb = (material.occlusionTextureID == NOT_PRESENT) ? ERROR_MAGENTA.rgb : texture(textures[material.occlusionTextureID], inUV).ggg;
                break;
            case 4:
                outColor.rgb = (material.emissiveTextureID == NOT_PRESENT) ? ERROR_MAGENTA.rgb : texture(textures[material.emissiveTextureID], inUV).rgb;
                break;
            case 5:
                outColor.rgb = (material.physicalDescriptorTextureID == NOT_PRESENT) ? ERROR_MAGENTA.rgb : texture(textures[material.physicalDescriptorTextureID], inUV).ggg;
                break;
            case 6:
                outColor.rgb = (material.physicalDescriptorTextureID == NOT_PRESENT) ? ERROR_MAGENTA.rgb : texture(textures[material.physicalDescriptorTextureID], inUV).aaa;
                break;
        }
        outColor = outColor;
    }
}
