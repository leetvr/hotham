struct QuadricData {
    mat4 gosFromLocal;
    mat4 surfaceQ;
    mat4 boundsQ;
    mat4 uvFromGos;
};

layout (std430, set = 1, binding = 0) readonly buffer QuadricDataBuffer {
    QuadricData data[];
} quadricDataBuffer;
