import json
import math

from sympy import false

angles = range(61)
seconds_per_degree = 24 * 60 * 60 / 360
steps_per_rotation = 200 * 16 # 16x microstepping
rotations_per_angle = []
total = 0

def rotations_from_alpha(alpha):
    a = 300
    b = 300
    rotation_height = 1.25 # mm M8 screw
    alpha_rad = alpha * (math.pi / 180)
    s_height = math.sqrt((a * math.sin(alpha_rad))**2 + (a - b * math.cos(alpha_rad))**2)
    return s_height / rotation_height

def speed_present(speed):
    for data in rotations_per_angle:
        if data.get('steps_per_second') == speed:
            return True
    return False


for alpha in angles:
    overall_rotations = rotations_from_alpha(alpha)
    rotations = overall_rotations - total
    if rotations == 0:
        continue
    steps = steps_per_rotation * rotations
    steps_per_seconds = round(steps / seconds_per_degree)
    if not speed_present(steps_per_seconds):
        rotations_per_angle.append(
            {
                'angle': alpha,
                'rotations': int(overall_rotations),
                'offset_steps': round((overall_rotations - int(overall_rotations)) * steps_per_rotation),
                'steps_per_second': steps_per_seconds
            }
        )
    total += rotations

with open("rotation_speeds.json", "w") as file:
    file.write(json.dumps(rotations_per_angle, indent=4))
