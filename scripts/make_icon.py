#!/usr/bin/env python3
"""Generate a 部-character logo: menu TIFF + .icns.

Menu bar icon must match Apple 拼 (pinyin.tiff): black glyph on a fully
transparent canvas, 16@1x + 32@2x template TIFF. Never punch 部 out of a
filled rounded plate — template inversion turns that plate into a blank
white rounded square in the dark input menu.
"""
from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RES = ROOT / "resources"
SWIFT = ROOT / "scripts" / "render_glyph.swift"


def swift_env() -> dict[str, str]:
    env = os.environ.copy()
    developer = env.get(
        "DEVELOPER_DIR", "/Applications/Xcode.app/Contents/Developer"
    )
    env["DEVELOPER_DIR"] = developer
    sdk = Path(developer) / "Platforms/MacOSX.platform/Developer/SDKs/MacOSX26.5.sdk"
    if not sdk.is_dir():
        sdk = Path(developer) / "Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk"
    env["SDKROOT"] = str(sdk)
    return env


def render(size: int, dest: Path, mode: str = "app") -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ["swift", str(SWIFT), str(size), str(dest), mode],
        check=True,
        capture_output=True,
        env=swift_env(),
    )


def verify_menu_template(path: Path) -> None:
    """Reject a filled plate. Corners transparent; 部 is black ink."""
    from PIL import Image

    im = Image.open(path)
    pages = getattr(im, "n_frames", 1)
    for i in range(pages):
        im.seek(i)
        px = im.convert("RGBA")
        width, height = px.size
        for xy in ((0, 0), (width - 1, 0), (0, height - 1), (width - 1, height - 1)):
            red, green, blue, alpha = px.getpixel(xy)
            if alpha > 24:
                raise SystemExit(
                    f"{path} page {i} corner {xy} must be transparent, "
                    f"got {(red, green, blue, alpha)}"
                )
        ink = 0
        plate_ring = 0
        ring_n = 0
        for y in range(height):
            for x in range(width):
                red, green, blue, alpha = px.getpixel((x, y))
                if alpha > 180 and red < 40 and green < 40 and blue < 40:
                    ink += 1
                if x < 2 or y < 2 or x >= width - 2 or y >= height - 2:
                    ring_n += 1
                    if alpha > 200:
                        plate_ring += 1
        if ink < max(18, (width * height) // 20):
            raise SystemExit(f"{path} page {i} has no black 部 ink ({ink} px)")
        if ring_n and plate_ring / ring_n > 0.78:
            raise SystemExit(
                f"{path} page {i} looks like a filled rounded plate "
                f"({plate_ring}/{ring_n} outer pixels opaque)"
            )
    print(f"verified {path}: transparent corners, black 部, no plate")


def main() -> None:
    RES.mkdir(exist_ok=True)
    src = RES / "icon.png"
    render(1024, src)

    with tempfile.TemporaryDirectory() as tmp:
        iconset = Path(tmp) / "icon.iconset"
        iconset.mkdir()
        sizes = [
            (16, "icon_16x16.png"),
            (32, "icon_16x16@2x.png"),
            (32, "icon_32x32.png"),
            (64, "icon_32x32@2x.png"),
            (128, "icon_128x128.png"),
            (256, "icon_128x128@2x.png"),
            (256, "icon_256x256.png"),
            (512, "icon_256x256@2x.png"),
            (512, "icon_512x512.png"),
            (1024, "icon_512x512@2x.png"),
        ]
        for size, name in sizes:
            render(size, iconset / name)
        subprocess.run(
            ["iconutil", "-c", "icns", str(iconset), "-o", str(RES / "icon.icns")],
            check=True,
            capture_output=True,
        )

    # Menu bar / input-source picker: 16@1x + 32@2x template TIFF, like Apple 拼.
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        png16 = tmp_path / "menu.png"
        png32 = tmp_path / "menu@2x.png"
        tiff16 = tmp_path / "menu16.tiff"
        tiff32 = tmp_path / "menu32.tiff"
        render(16, png16, "menu")
        render(32, png32, "menu")
        subprocess.run(
            [
                "sips",
                "-s",
                "format",
                "tiff",
                "-s",
                "dpiWidth",
                "72",
                "-s",
                "dpiHeight",
                "72",
                str(png16),
                "--out",
                str(tiff16),
            ],
            check=True,
            capture_output=True,
        )
        subprocess.run(
            [
                "sips",
                "-s",
                "format",
                "tiff",
                "-s",
                "dpiWidth",
                "144",
                "-s",
                "dpiHeight",
                "144",
                str(png32),
                "--out",
                str(tiff32),
            ],
            check=True,
            capture_output=True,
        )
        menu = RES / "menu.tiff"
        subprocess.run(
            ["tiffutil", "-cat", str(tiff16), str(tiff32), "-out", str(menu)],
            check=True,
            capture_output=True,
        )
        shutil.copyfile(menu, RES / "icon.tiff")
        verify_menu_template(menu)
    print("wrote icon.icns menu.tiff")


if __name__ == "__main__":
    main()
