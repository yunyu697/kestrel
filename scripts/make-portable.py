#!/usr/bin/env python3
"""Build the portable (green) release zip from the compiled binary.

The Tauri binary is self-contained: the frontend is embedded in the exe via the
custom protocol, so a portable build is just the exe plus a short readme.

Usage:  python scripts/make-portable.py [--out dist-release]
"""
import argparse
import json
import os
import shutil
import sys
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXE = os.path.join(ROOT, "src-tauri", "target", "release", "kestrel.exe")

README = """Kestrel {version} —— 便携版（免安装）
=====================================

运行方式
  解压后直接双击 kestrel.exe 即可，不需要安装。

快捷键
  双击 Ctrl      打开 / 关闭搜索栏
  在打开/保存对话框中按 Ctrl+G   吸附搜索栏到对话框下方
  搜索栏内：↑↓ 选择、回车打开、Tab 操作菜单、Esc 关闭
  快速切换（吸附后）：输入 .. 回上级、D: 切盘、~ 回用户目录，
  ← / → 在目录层级间移动，跳转只改对话框位置、不替你确认

运行要求
  Windows 10 1809 或更高版本（64 位）。
  依赖 Microsoft Edge WebView2 运行时：Windows 11 已内置；Windows 10 如果
  没有，请安装微软官方的 "WebView2 Runtime"（Evergreen Bootstrapper）。

数据存放位置
  配置、索引、剪贴板历史都在 %APPDATA%\\com.kestrel.launcher
  换机器时把这个文件夹一起拷走，就能保留设置和历史。

卸载
  关闭程序后删除本文件夹即可。如果在设置里开启过"开机启动"，请先在设置里
  关闭，或删除注册表 HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run
  下的 Kestrel 项。
"""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=os.path.join(ROOT, "release"))
    args = ap.parse_args()

    if not os.path.isfile(EXE):
        print(f"missing binary: {EXE}\nrun `npx tauri build` first", file=sys.stderr)
        return 1

    info = {}
    p = os.path.join(ROOT, "build-info.json")
    if os.path.isfile(p):
        info = json.load(open(p, encoding="utf-8"))
    version = info.get("version", "1.0.0")
    build = info.get("build")

    name = f"Kestrel-{version}-portable-win-x64"
    os.makedirs(args.out, exist_ok=True)
    staging = os.path.join(args.out, name)
    if os.path.isdir(staging):
        shutil.rmtree(staging)
    os.makedirs(staging)

    shutil.copy2(EXE, os.path.join(staging, "kestrel.exe"))
    with open(os.path.join(staging, "README.txt"), "w", encoding="utf-8-sig") as f:
        f.write(README.format(version=version))

    zip_path = os.path.join(args.out, f"{name}.zip")
    if os.path.exists(zip_path):
        os.remove(zip_path)
    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        for base, _dirs, files in os.walk(staging):
            for fn in files:
                full = os.path.join(base, fn)
                rel = os.path.relpath(full, staging)
                z.write(full, os.path.join(name, rel))

    size = os.path.getsize(zip_path)
    print(f"portable: {zip_path}  ({size / 1048576:.1f} MB, build {build})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
