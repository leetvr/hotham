#version 460
#extension GL_EXT_nonuniform_qualifier:enable
#extension GL_ARB_separate_shader_objects : enable

// Inputs
layout (location = 0) in vec4 inGosPos;
layout (location = 1) in vec2 inUV;
layout (location = 2) flat in uint inMaterialID;
layout (location = 3) in vec3 inNormal;

// Textures
layout (set = 0, binding = 4) uniform sampler2D textures[];
layout (set = 0, binding = 5) uniform samplerCube cubeTextures[];

layout( push_constant ) uniform constants
{
    uint baseColorTextureID;
} PushConstants;

// Outputs
layout (location = 0) out vec4 outColor;

struct Light {
    vec3 direction;
    float range;

    vec3 color;
    float intensity;

    vec3 position;
    float innerConeCos;

    float outerConeCos;
    uint type;
};

const uint LightType_Directional = 0;
const uint LightType_Point = 1;
const uint LightType_Spot = 2;

layout (set = 0, binding = 3) readonly uniform SceneData {
    mat4 viewProjection[2];
    vec4 cameraPosition[2];
    vec4 params;
    Light lights[4];
} sceneData;


// The world's worst shader (TM).
//
// Takes all the textures and blends them into a slurry. Gives no fucks whether the texture is actually there.
void main() {
    // outColor = sceneData.params * sceneData.viewProjection[0] * sceneData.viewProjection[1] * sceneData.lights[0].range * sceneData.lights[1].range;
    outColor = vec4(1.0);

}