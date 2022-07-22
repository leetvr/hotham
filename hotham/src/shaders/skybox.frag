#version 450
#extension GL_EXT_multiview : enable

layout (location = 0) in vec2 screenCoords;
layout (location = 0) out vec4 outFragColor;

layout(push_constant) uniform block {
    mat4 views[2];
} pushConstant;

void main() 
{
  //  Just set the out colour to be the coordinates of this fragment.
  const vec4 viewDirHomogenous = pushConstant.views[gl_ViewIndex] * vec4(2 * screenCoords - 1, 0, 1);
  outFragColor = viewDirHomogenous;
}