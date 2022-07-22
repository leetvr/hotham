#version 460

layout(location = 0) out vec2 screenCoords;

// Render a triangle that covers the entire screen.
// Code generously donated by Benjamin Saunders (@Ralith)
void main() {
    screenCoords = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);
    gl_Position = vec4(screenCoords * 2.0f + -1.0f, 1.0, 1.0f);
}