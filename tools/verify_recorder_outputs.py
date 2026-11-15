#!/usr/bin/env python3
"""Validate --verify-recorder output with ffprobe and Pillow (test-only dependency)."""
import json
from pathlib import Path
import subprocess
import sys

from PIL import Image, ImageSequence

directory = Path(sys.argv[1]).resolve()
for entry in json.loads((directory / "recordings.json").read_text()):
    path = Path(entry["path"])
    if not path.is_absolute():
        path = Path.cwd() / path
    probe = json.loads(subprocess.check_output([
        "ffprobe", "-v", "error", "-count_frames", "-show_streams", "-of", "json", str(path)
    ]))["streams"][0]
    assert int(probe["nb_read_frames"]) == entry["frames"], (entry, probe)
    assert (probe["width"], probe["height"]) == (entry["width"], entry["height"])
    frames, durations = [], []
    with Image.open(path) as image:
        assert image.info.get("loop") == 0, image.info
        for frame in ImageSequence.Iterator(image):
            rgba = frame.convert("RGBA")
            alpha = rgba.getchannel("A")
            if entry["transparent"]:
                assert alpha.getextrema() == (0, 255), (path, "Missing alpha or subject")
                assert rgba.getpixel((0, 0))[3] == 0, (path, "Opaque backdrop")
            else:
                assert alpha.getextrema() == (255, 255), (path, "Unexpected transparency")
            durations.append(frame.info["duration"] / 1000)
            frames.append(rgba.tobytes())
    assert len(set(frames)) > 1, (path, "Animation is frozen")
    assert all(abs(value - 1 / entry["fps"]) < 0.00001 for value in durations), durations
    actual = sum(durations)
    assert -0.00001 <= actual - entry["duration"] < 1 / entry["fps"] + 0.00001, entry
    print(f"PASS {path.name}: {len(frames)} frames, {actual:.3f}s, {entry['fps']} fps, transparent={entry['transparent']}")

assert not list(directory.glob(".recording-*")), "Temporary jobs were not cleaned up"
for index, entry in enumerate(json.loads((directory / "recordings.json").read_text())):
    with Image.open(directory / f"photo-{index}.png") as image:
        assert image.size == (entry["width"], entry["height"])
        assert image.getchannel("A").getextrema() == ((0, 255) if entry["transparent"] else (255, 255))
print("PASS PNG dimensions/alpha and recording cleanup")
