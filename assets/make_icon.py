"""Export the app's PNG and Windows ICO assets from the approved icon source.

Run from any directory: python3 /path/to/meatshell/assets/make_icon.py
Requires Pillow. The original artwork stays in icon-source.png.
"""

from pathlib import Path

from PIL import Image


ASSET_DIR = Path(__file__).resolve().parent
PNG_SIZES = {256: "icon.png", 512: "icon@512.png", 1024: "icon@1024.png"}
ICO_SIZES = [(size, size) for size in (16, 24, 32, 48, 64, 128, 256)]


def main() -> None:
    with Image.open(ASSET_DIR / "icon-source.png") as source:
        if source.width != source.height:
            raise ValueError("The app icon source must be square")
        master = source.convert("RGBA")

    for size, name in PNG_SIZES.items():
        master.resize((size, size), Image.Resampling.LANCZOS).save(ASSET_DIR / name)

    master.save(ASSET_DIR / "meatshell.ico", format="ICO", sizes=ICO_SIZES)
    print("Generated icon.png, icon@512.png, icon@1024.png and meatshell.ico")


if __name__ == "__main__":
    main()
