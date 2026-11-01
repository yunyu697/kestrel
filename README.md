<div align="center">

<img src="icon-1024.png" width="128" alt="Kestrel 图标"/>

# Kestrel

**Windows 上的极速文件搜索与应用启动器**

双击 <kbd>Ctrl</kbd>，瞬时定位文件、启动应用、执行命令。

简体中文 · 繁體中文 · English · 日本語 · Español

[下载最新版](https://github.com/yunyu697/kestrel/releases/latest) · [全部版本](https://github.com/yunyu697/kestrel/releases) · [问题反馈](https://github.com/yunyu697/kestrel/issues)

</div>

---

## 特性

### 极速搜索

- 文件系统**全盘索引**，输入即出结果，支持**拼音**、**模糊匹配**与**路径匹配**
- 内置计算器、剪贴板历史、窗口切换器与自定义命令
- 索引压缩存储，磁盘占用小
- 界面语言支持**跟随系统**，切换 Windows 语言后自动匹配

### 快速切换（Quick Switch）

在任意「打开 / 保存」对话框中按 <kbd>Ctrl</kbd>+<kbd>G</kbd>，搜索栏会吸附到对话框下方，帮你把文件对话框**导航到目标目录**——只跳转、不确认，用不用这个目录由你决定：

- **路径语法**：`..` 上跳一级、`\` 到盘根、`D:` 直接切盘、`~` 回到用户目录，半截名字自动补全
- **层级导航**：<kbd>←</kbd> / <kbd>→</kbd> 在目录层级间来回走，连续下钻不必重新输入

应用自己画的「选择文件夹」对话框（例如 WinRAR 的「解压路径和选项」）同样支持：选中文件夹后是**在它的文件夹树里选中那个节点**，路径框由对话框自己同步，不需要往输入框里打字。

### 可定制的界面

- 预览面板可停靠右侧或底部
- 颜色主题可分别调整窗口、卡片、面板、三档文字、描边与选中色
- 主快捷键提供常用预设，一键套用；收藏夹内置桌面、下载、文档等固定项

### 检查更新

- 可设置检查频率（每天 / 每周 / 从不），支持启动时立即检查
- 发现新版本时启动器顶部出现提示条，点击直达下载
- 内置多个 GitHub 加速**镜像**并可一键测速，也支持自定义镜像前缀（适合国内网络）

## 下载

前往 [Releases](https://github.com/yunyu697/kestrel/releases/latest) 页面，按需选择：

| 资产 | 说明 |
|---|---|
| `Kestrel_x.y.z_x64_zh-CN.msi` | Windows 安装包（推荐，MSI 格式） |
| `Kestrel_x.y.z_x64-setup.exe` | Windows 安装包（NSIS 格式，按当前用户安装） |
| `Kestrel_x.y.z-portable-win-x64.zip` | 便携版，解压即用，无需安装 |

系统要求：Windows 10/11（x64）。

## 使用

| 快捷键 | 功能 |
|---|---|
| 双击 <kbd>Ctrl</kbd> | 唤起 / 隐藏启动器 |
| <kbd>Ctrl</kbd>+<kbd>G</kbd> | 在打开 / 保存对话框中唤出快速切换栏 |
| <kbd>Ctrl</kbd>+<kbd>L</kbd> | （快速切换栏内）直接输入路径导航 |

托盘图标：单击唤起启动器，双击打开设置。

## 从源码构建

环境要求：**Node.js 22+**、**Rust（stable，MSVC 工具链）**、Windows 10/11。

```bash
npm ci
npm run app:build   # 产出 MSI / NSIS 安装包到 src-tauri/target/release/bundle
```

开发模式：

```bash
npm run app:dev
```

其他脚本：

| 命令 | 说明 |
|---|---|
| `npm run build` | 仅构建前端（含类型检查） |
| `node scripts/check-i18n.mjs` | 校验四语字典键完整性 |
| `node scripts/make-icon.mjs` | 重新生成应用图标 |
| `python scripts/make-portable.py <version>` | 生成便携版 zip |

## 项目结构

```
kestrel/
├── src/                  # React 前端（启动器、设置界面、共享逻辑）
│   ├── launcher/         #   启动器主窗口（搜索、路径导航、更新提示条）
│   ├── settings/         #   设置窗口（常规、外观、热键、关于）
│   └── shared/           #   i18n 字典、路径解析、更新检查、发布日志
├── src-tauri/            # Rust 后端（Tauri 2）
│   └── src/
│       ├── win/          #   Win32 集成：索引、钩子、对话框监控、窗口管理
│       └── ...           #   设置持久化、命令、托盘等
├── scripts/              # 构建/校验辅助脚本
└── .github/workflows/    # CI（Windows：tsc + i18n 校验 + cargo check/test）
```

## 版本历史

每个版本的完整改动可见 [Releases](https://github.com/yunyu697/kestrel/releases)，也可在应用内「设置 → 关于」中直接查看。

| 版本 | 日期 | 亮点 |
|---|---|---|
| 1.2.6 | 2026-11-01 | 同一时间只允许运行一个实例，重复点击图标会唤出搜索栏 |
| 1.2.5 | 2026-11-01 | 快速切换支持应用自己的选文件夹对话框，在文件夹树里直接跳转 |
| 1.2.0 | 2026-10-31 | 检查更新：自动发现新版本，镜像加速下载 |
| 1.1.0 | 2026-10-31 | 快速切换：路径语法导航，跳转不替你确认 |
| 1.0.0 | 2026-10-30 | 界面深度定制、高 DPI 修复、更稳的运行 |
| 0.9.0 | 2026-10-28 | 收藏夹固定项、快捷键预设、四语界面 |
| 0.8.0 | 2026-10-27 | 快速切换吸附文件对话框（Ctrl+G） |
| 0.7.0 | 2026-10-26 | 首个版本：全盘索引、拼音/模糊搜索、内置工具 |

## 许可证

本项目尚未附带开源许可证，暂保留所有权利。
