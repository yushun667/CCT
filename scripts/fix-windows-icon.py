#!/usr/bin/env python3
"""
生成 Windows RC 兼容的 icon.ico（3.00 / BMP 格式）。

RC.EXE 不接受内含 PNG 的 ICO，需使用 BMP 格式。
用法（在仓库根目录）:
  pip install Pillow
  python scripts/fix-windows-icon.py
"""
from pathlib import Path

def main():
    try:
        from PIL import Image
    except ImportError:
        print("请先安装 Pillow: pip install Pillow")
        raise SystemExit(1)

    root = Path(__file__).resolve().parent.parent
    src = root / "src-tauri" / "icons" / "32x32.png"
    dst = root / "src-tauri" / "icons" / "icon.ico"

    if not src.exists():
        print(f"未找到 {src}")
        raise SystemExit(1)

    img = Image.open(src).convert("RGBA")
    sizes = [(16, 16), (32, 32), (48, 48), (64, 64)]
    img.save(dst, format="ICO", sizes=sizes, bitmap_format="bmp")
    print(f"已写入 {dst} (BMP 格式，供 Windows RC 使用)")

if __name__ == "__main__":
    main()
