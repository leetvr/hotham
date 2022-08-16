#version 460

#include "common.glsl"

layout (location = 0) in vec3 inPos;

layout (location = 0) out vec4 outRayOrigin;
layout (location = 1) out vec4 outRayDir;
layout (location = 2) out vec4 outSurfaceQTimesRayOrigin;
layout (location = 3) out vec4 outSurfaceQTimesRayDir;
layout (location = 4) out flat uint outInstanceIndex;

layout (std430, set = 0, binding = 6) readonly buffer QuadricDataBuffer {
    QuadricData data[];
} quadricDataBuffer;

out gl_PerVertex {
    vec4 gl_Position;
};

void main() {
    QuadricData d = quadricDataBuffer.data[gl_InstanceIndex];
    outInstanceIndex = gl_InstanceIndex;

    outRayOrigin = d.globalFromLocal * vec4(inPos, 1.0);
    outRayDir = vec4((outRayOrigin.xyz / outRayOrigin.w) - sceneData.cameraPosition[gl_ViewIndex].xyz, 0.0);

    outSurfaceQTimesRayOrigin = d.surfaceQ * outRayOrigin;
    outSurfaceQTimesRayDir = d.surfaceQ * outRayDir;

    gl_Position = sceneData.viewProjection[gl_ViewIndex] * outRayOrigin;
}
