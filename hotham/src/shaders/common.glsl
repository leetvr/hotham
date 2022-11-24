#extension GL_ARB_separate_shader_objects : enable

#define NOT_PRESENT 4294967295
#define MAX_JOINTS 64

#define PI                 F16(3.14159265359)
#define HALF_PI            F16(1.570796327)
#define MEDIUMP_FLT_MAX    F16(65504.0)
#define MEDIUMP_FLT_MIN    F16(0.00006103515625)
#define saturateMediump(x) min(x, MEDIUMP_FLT_MAX)
#define F16(x)             float16_t(x)
#define V16(x)             f16vec3(x)
#define saturate(x)        clamp(x, F16(0), F16(1))

// Representation of a light in a scene, based on the KHR_lights_punctual extension:
// https://github.com/KhronosGroup/glTF/tree/master/extensions/2.0/Khronos/KHR_lights_punctual
// TODO: make these f16
struct Light {
    vec3 direction;
    float falloff;

    vec3 color;
    float intensity;

    vec3 position;
    float lightAngleScale;

    float lightAngleOffset;
    uint type;
};

const uint LightType_Directional = 0;
const uint LightType_Point = 1;
const uint LightType_Spot = 2;

layout (set = 0, binding = 2) readonly uniform SceneData {
    mat4 viewProjection[2];
    vec4 cameraPosition[2];
    vec4 params;
    Light lights[4];
} sceneData;
