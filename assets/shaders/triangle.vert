#version 330 core
layout (location = 0) in vec3 pos;

uniform float timeSinceLastFrame;
uniform float elapsedTime;

mat2 rotate2D(float angle) {
    return mat2(cos(angle), -sin(angle), sin(angle), cos(angle));
}

void main() {
    vec2 rotatedXY = rotate2D(mod(elapsedTime, 360)) * pos.xy;

    float deltaX = sin(elapsedTime) / 2;

    gl_Position = vec4(deltaX + rotatedXY.x, rotatedXY.y, pos.z, 1.0);
}
