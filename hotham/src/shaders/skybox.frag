#version 450
#extension GL_EXT_multiview : enable

layout (location = 0) in vec2 screenCoords;
layout (location = 0) out vec4 outFragColor;

layout(push_constant) uniform block {
    mat4 inverseProjection;
    mat4 viewToWorld;
} pushConstant;

void main() 
{
  //  Just set the out colour to be the coordinates of this fragment.
  const vec4 viewDirHomogenous = pushConstant.inverseProjection * vec4(2 * screenCoords - 1, 0, 1);
  const vec4 viewDirWorldSpace = pushConstant.viewToWorld * viewDirHomogenous; 
  outFragColor = viewDirWorldSpace;
}