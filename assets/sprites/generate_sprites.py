#!/usr/bin/env python3
"""Generate placeholder sprite sheet for Pet Bolha"""

from PIL import Image, ImageDraw
import os

OUTPUT_DIR = r"C:\Users\mathe\Hermes_Brain\10-Projetos\pet-bolha\assets\sprites"
FRAME_SIZE = 64
COLS = 8
ROWS = 4  # 32 frames total

def draw_pet_frame(draw, x, y, frame_type, frame_idx):
    """Draw a single frame of the pet"""
    cx, cy = x + 32, y + 32
    
    # Body color based on type
    colors = {
        'idle': (100, 200, 255),
        'walk': (80, 180, 235),
        'sleep': (120, 150, 200),
        'happy': (100, 255, 150),
        'sad': (150, 150, 200),
        'angry': (255, 100, 100),
        'eat': (255, 200, 100),
        'play': (255, 150, 200),
        'pet': (200, 255, 200),
        'think': (180, 180, 255),
    }
    
    base_color = colors.get(frame_type, (100, 200, 255))
    
    # Body (circle)
    draw.ellipse([cx-28, cy-28, cx+28, cy+28], fill=base_color, outline=(50, 100, 150), width=2)
    
    # Eyes based on frame type and index
    blink = (frame_idx % 8) == 0  # Blink every 8 frames
    
    if frame_type == 'sleep':
        # Closed eyes
        draw.line([cx-15, cy-5, cx-5, cy-5], fill=(20, 20, 50), width=2)
        draw.line([cx+5, cy-5, cx+15, cy-5], fill=(20, 20, 50), width=2)
    elif blink:
        # Blinking
        draw.line([cx-15, cy-5, cx-5, cy-5], fill=(20, 20, 50), width=3)
        draw.line([cx+5, cy-5, cx+15, cy-5], fill=(20, 20, 50), width=3)
    elif frame_type == 'happy':
        # Happy eyes (curved up)
        draw.arc([cx-15, cy-10, cx-5, cy], start=0, end=180, fill=(20, 20, 50), width=2)
        draw.arc([cx+5, cy-10, cx+15, cy], start=0, end=180, fill=(20, 20, 50), width=2)
    elif frame_type == 'sad':
        # Sad eyes (droopy)
        draw.arc([cx-15, cy-5, cx-5, cy+5], start=180, end=360, fill=(20, 20, 50), width=2)
        draw.arc([cx+5, cy-5, cx+15, cy+5], start=180, end=360, fill=(20, 20, 50), width=2)
    elif frame_type == 'angry':
        # Angry eyes (angled down)
        draw.line([cx-15, cy-10, cx-5, cy], fill=(20, 20, 50), width=3)
        draw.line([cx+5, cy-10, cx+15, cy], fill=(20, 20, 50), width=3)
    else:
        # Normal eyes
        draw.ellipse([cx-12, cy-8, cx-6, cy-2], fill=(20, 20, 50))
        draw.ellipse([cx+6, cy-8, cx+12, cy-2], fill=(20, 20, 50))
        # Pupils
        draw.ellipse([cx-10, cy-7, cx-8, cy-5], fill=(255, 255, 255))
        draw.ellipse([cx+8, cy-7, cx+10, cy-5], fill=(255, 255, 255))
    
    # Mouth
    if frame_type == 'happy':
        draw.arc([cx-10, cy+5, cx+10, cy+20], start=0, end=180, fill=(20, 20, 50), width=2)
    elif frame_type == 'sad':
        draw.arc([cx-10, cy+10, cx+10, cy+25], start=180, end=360, fill=(20, 20, 50), width=2)
    elif frame_type == 'eat':
        draw.ellipse([cx-8, cy+10, cx+8, cy+18], fill=(20, 20, 50))
    elif frame_type == 'angry':
        draw.line([cx-10, cy+15, cx+10, cy+15], fill=(20, 20, 50), width=2)
    elif frame_type == 'sleep':
        draw.arc([cx-5, cy+10, cx+5, cy+15], start=180, end=360, fill=(20, 20, 50), width=1)
    else:
        draw.line([cx-8, cy+12, cx+8, cy+12], fill=(20, 20, 50), width=1)
    
    # Animation-specific details
    if frame_type == 'walk':
        # Legs moving
        leg_offset = 5 if frame_idx % 2 == 0 else -5
        draw.line([cx-10, cy+28, cx-10+leg_offset, cy+38], fill=(50, 100, 150), width=3)
        draw.line([cx+10, cy+28, cx+10-leg_offset, cy+38], fill=(50, 100, 150), width=3)
    elif frame_type == 'play':
        # Ball
        ball_x = cx + 30 + (frame_idx % 4) * 5
        ball_y = cy + 10
        draw.ellipse([ball_x-8, ball_y-8, ball_x+8, ball_y+8], fill=(255, 200, 50), outline=(200, 150, 0), width=2)
    elif frame_type == 'pet':
        # Heart particles
        for i in range(3):
            hx = cx - 20 + i * 15
            hy = cy - 30 - (frame_idx % 4) * 3
            draw.text((hx, hy), "♥", fill=(255, 100, 150))
    elif frame_type == 'think':
        # Thought bubble
        draw.ellipse([cx+25, cy-35, cx+45, cy-15], fill=(255, 255, 255, 200), outline=(100, 100, 100), width=1)
        draw.ellipse([cx+30, cy-30, cx+35, cy-25], fill=(200, 200, 255))
        draw.ellipse([cx+35, cy-28, cx+39, cy-24], fill=(200, 200, 255))

def generate_sprite_sheet(name):
    """Generate a complete sprite sheet for a personality"""
    # Layout: 8 columns x 4 rows = 32 frames
    # Row 0: idle (4), walk (4)
    # Row 1: sleep (2), happy (3), sad (2), angry (3)
    # Row 2: eat (2), play (4), pet (2), think (2)
    # Row 3: more variations...
    
    frame_sequence = [
        # Row 0
        ('idle', 0), ('idle', 1), ('idle', 2), ('idle', 3),
        ('walk', 0), ('walk', 1), ('walk', 2), ('walk', 3),
        # Row 1
        ('sleep', 0), ('sleep', 1),
        ('happy', 0), ('happy', 1), ('happy', 2),
        ('sad', 0), ('sad', 1),
        ('angry', 0), ('angry', 1), ('angry', 2),
        # Row 2
        ('eat', 0), ('eat', 1),
        ('play', 0), ('play', 1), ('play', 2), ('play', 3),
        ('pet', 0), ('pet', 1),
        ('think', 0), ('think', 1),
        # Row 3 - more variations
        ('idle', 4), ('idle', 5), ('idle', 6), ('idle', 7),
        ('walk', 4), ('walk', 5), ('walk', 6), ('walk', 7),
    ]
    
    sheet_width = COLS * FRAME_SIZE
    sheet_height = ROWS * FRAME_SIZE
    sheet = Image.new('RGBA', (sheet_width, sheet_height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(sheet)
    
    for idx, (frame_type, frame_idx) in enumerate(frame_sequence):
        col = idx % COLS
        row = idx // COLS
        x = col * FRAME_SIZE
        y = row * FRAME_SIZE
        draw_pet_frame(draw, x, y, frame_type, frame_idx)
    
    # Save
    os.makedirs(OUTPUT_DIR, exist_ok=True)
    output_path = os.path.join(OUTPUT_DIR, f"{name}.png")
    sheet.save(output_path)
    print(f"Generated {output_path} ({sheet_width}x{sheet_height})")
    
    # Also save frame map JSON
    import json
    frame_map = {}
    animations = {
        'idle': [0, 1, 2, 3, 32, 33, 34, 35],
        'walk': [4, 5, 6, 7, 36, 37, 38, 39],
        'sleep': [8, 9],
        'happy': [10, 11, 12],
        'sad': [13, 14],
        'angry': [15, 16, 17],
        'eat': [18, 19],
        'play': [20, 21, 22, 23],
        'pet': [24, 25],
        'think': [26, 27],
    }
    
    frame_map['frame_size'] = [FRAME_SIZE, FRAME_SIZE]
    frame_map['animations'] = animations
    frame_map['cols'] = COLS
    frame_map['rows'] = ROWS
    
    json_path = os.path.join(OUTPUT_DIR, f"{name}_frames.json")
    with open(json_path, 'w') as f:
        json.dump(frame_map, f, indent=2)
    print(f"Generated {json_path}")

if __name__ == '__main__':
    for name in ['bolha', 'bryan', 'jorge']:
        generate_sprite_sheet(name)