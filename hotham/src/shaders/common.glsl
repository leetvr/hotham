#extension GL_EXT_nonuniform_qualifier:enable
#extension GL_ARB_separate_shader_objects : enable
#extension GL_EXT_multiview : enable

#define NOT_PRESENT 4294967295
#define MAX_JOINTS 64

struct DrawData {
    mat4 globalFromLocal;
    mat4 localFromGlobal;
    uint materialID;
    uint skinID;
};

struct QuadricData {
    mat4 globalFromLocal;
    mat4 surfaceQ;
    mat4 boundsQ;
    mat4 uvFromGlobal;
    uint materialID;
};

// Representation of a light in a scene, based on the KHR_lights_punctual extension:
// https://github.com/KhronosGroup/glTF/tree/master/extensions/2.0/Khronos/KHR_lights_punctual
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
