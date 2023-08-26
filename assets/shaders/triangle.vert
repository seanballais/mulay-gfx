#version 330 core
layout (location = 0) in vec3 pos;

uniform float timeSinceLastFrame;
uniform float elapsedTime;

void main() {
    float deltaX = sin(elapsedTime) / 2;
    float deltaY = cos(elapsedTime) / 2;

    gl_Position = vec4(deltaX + pos.x, deltaY + pos.y, pos.z, 1.0);
}
