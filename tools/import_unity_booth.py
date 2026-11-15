#!/usr/bin/env python3
"""Copy reference assets into this project; the Unity source is always read-only.

Usage: python3 tools/import_unity_booth.py /path/to/KanadeSA
No Unity/Python packages are required. Sprite metadata remains beside each booth.
"""
import json
import re
import shutil
import struct
import sys
from pathlib import Path
from compact_json import compact_json

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets"
SOURCE = Path(sys.argv[1]).resolve() / "Assets"


def copy(source, target):
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, target)


def write_json(target, value):
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(
        json.dumps(value, ensure_ascii=False, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )


def png_size(path):
    with path.open("rb") as image:
        return struct.unpack(">II", image.read(24)[16:24])


def metadata(path):
    text = path.with_suffix(path.suffix + ".meta").read_text()

    def value(key, fallback):
        match = re.search(r"^\s*" + key + r": ([^\n]+)", text, re.M)
        return float(match[1]) if match else fallback

    pivot = re.search(r"spritePivot: \{x: ([^,]+), y: ([^}]+)\}", text)
    return {
        "size": png_size(path),
        "pixels_per_unit": value("spritePixelsToUnits", 100),
        "pivot": [float(pivot[1]), float(pivot[2])] if pivot else [0.5, 0.5],
        "filter_mode": int(value("filterMode", 1)),
        "mipmaps": bool(value("enableMipMap", 0)),
        "srgb": bool(value("sRGBTexture", 1)),
        "alpha_is_transparency": bool(value("alphaIsTransparency", 1)),
        "wrap_mode": [int(value("wrapU", 1)), int(value("wrapV", 1))],
        "sprite_alignment": int(value("alignment", 0)),
        "mesh_type": int(value("spriteMeshType", 0)),
        "max_texture_size": int(value("maxTextureSize", 2048)),
    }


configs = SOURCE / "Core/Configs"
for name in ("pets", "umbrellas", "emotes", "boothbg"):
    target = ASSETS / "configs" / f"_{name}.json"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(
        compact_json((configs / f"_{name}.json").read_text(encoding="utf-8")),
        encoding="utf-8",
    )

booth = SOURCE / "Resources/booth"
groups = {}
for source in sorted(booth.rglob("*.png")):
    relative = source.relative_to(booth).as_posix().lower()
    copy(source, ASSETS / "booth" / relative)
    parent = str(Path(relative).parent)
    groups.setdefault(parent, {})[Path(relative).name] = metadata(source)
for parent, sprites in groups.items():
    write_json(ASSETS / "booth" / parent / "sprites.json",
               {"format_version": 1, "sprites": sprites})

for source_name, destination in (
    ("mini animals.atlas.txt", "skeleton.atlas"),
    ("mini animals.skel.bytes", "skeleton.skel.bytes"),
    ("mini animals.png", "mini animals.png"),
):
    copy(SOURCE / "Core/Spine/Pets" / source_name, ASSETS / "spine/pets" / destination)

# A separate atlas uses the original untrimmed PNG rectangle. Attachment cloning
# keeps the template's geometry, dimensions and linked mesh deform identity.
atlas = []
for source in sorted((SOURCE / "Core/Spine/Character/Umbrellas/full").glob("*.png")):
    copy(source, ASSETS / "spine/umbrellas" / source.name)
    w, h = png_size(source)
    atlas.append(f"{source.name}\nsize:{w},{h}\nfilter:Linear,Linear\n"
                 f"{source.stem}\nbounds:0,0,{w},{h}\n")
(ASSETS / "spine/umbrellas/skeleton.atlas").write_text("\n".join(atlas))

for source in (SOURCE / "Core/Spine/Gems").glob("*.png"):
    copy(source, ASSETS / "booth/gems" / source.name)
for source_name, destination in (
    ("Core/Texture2D/Shadows/booth-char-shadow.png", "shadow.png"),
    ("Core/Texture2D/UGUI/Booth/BoothLogo.png", "watermark.png"),
):
    source = SOURCE / source_name
    copy(source, ASSETS / "booth" / destination)
    groups["."][destination] = metadata(source)
write_json(ASSETS / "booth/sprites.json",
           {"format_version": 1, "sprites": groups["."]})
write_json(ASSETS / "booth/gems/sprites.json", {
    "format_version": 1,
    "sprites": {p.name: metadata(p) for p in sorted((SOURCE / "Core/Spine/Gems").glob("*.png"))},
})

icon_paths = {"UI/InventoryIcons/item_rejection_letter"}
for name in ("weapons", "pets", "umbrellas", "emotes"):
    for item in json.loads((configs / f"_{name}.json").read_text()):
        if item.get("weaponClass") == "Gun":
            icon_paths.update(item["icon"] + skin for skin in item.get("skinNames", []))
        elif item.get("icon"):
            icon_paths.add(item["icon"])
resources = {p.relative_to(SOURCE / "Resources").as_posix().lower(): p
             for p in (SOURCE / "Resources").rglob("*.png")}
missing = []
for icon in sorted(icon_paths):
    key = icon.lower() + ".png"
    source = resources.get(key)
    if source is None:
        missing.append(key)
    else:
        copy(source, ASSETS / "icons" / key)
print(f"Imported {sum(map(len, groups.values()))} booth sprites, 139 umbrella pages, "
      f"pets, gems and {len(icon_paths) - len(missing)} equipment icons.")
if missing:
    print("Configured icons using Unity's rejection-letter fallback:\n" + "\n".join(missing))
