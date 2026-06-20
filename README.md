# NexPlay

NexPlay 是一个基于 Electron + React 前端、Rust 本地后端的番剧媒体库桌面应用。它面向本地番剧文件管理，支持媒体库扫描、Bangumi 元数据补全、dandanplay 弹幕匹配、剧集详情和本地播放入口。

## 功能状态

- Electron + React + Vite 渲染端
- Rust 后端负责配置、SQLite、媒体扫描、Bangumi/dandanplay 集成和 JSON-RPC 后台进程
- Electron IPC 连接前后端，不使用 HTTP 服务
- 支持 Windows、macOS、Linux 的 GitHub Actions 自动打包
- Linux/macOS release 构建会保留 libmpv 播放后端；Windows release 当前先提供媒体库和扫描等核心功能，内置播放器后端后续需要补齐 libmpv Windows 打包链路

## 本地运行

安装依赖：

```bash
npm install
```

启动开发模式：

```bash
npm run dev
```

`npm run dev` 会同时启动 Vite 渲染进程和 Electron 主进程。Rust 后端在开发模式下通过 `cargo run --quiet -- backend-daemon` 启动。

如果要使用 libmpv 播放相关能力，需要系统安装 mpv/libmpv 运行库；如果要编译原生渲染桥，还需要对应平台的 libmpv 开发文件和 `pkg-config`：

```bash
npm run build:native-render
```

## 本地构建

只构建渲染端并用 Electron 生产模式预览：

```bash
npm run build
npm start
```

构建当前平台安装包：

```bash
npm run dist
```

构建流程会执行：

1. `tsc --noEmit && vite build`
2. `cargo build --release`
3. `node scripts/prepare-release-assets.cjs`
4. `electron-builder`

产物输出到 `release/`。打包后的应用会内置 Rust 后端二进制，生产环境配置文件写入 Electron 的 userData 目录，而不是安装目录。

## GitHub Release

推送 `v*` 标签会自动触发跨平台打包并发布到 GitHub Release：

```bash
git tag v0.1.0
git push origin v0.1.0
```

也可以在 GitHub Actions 页面手动运行 `Release` workflow，只生成 workflow artifacts，不创建 GitHub Release。

自动构建的平台和主要产物：

- Windows: NSIS installer (`.exe`)
- macOS: `.dmg` 和 `.zip`
- Linux: `.AppImage`、`.deb`、`.tar.gz`

## 目录结构

- `electron/`: Electron 主进程、preload、IPC、播放器控制和资源协议
- `frontend/src/`: React 渲染端源码
- `src/`: Rust 后端，包含媒体库扫描、SQLite、Bangumi/dandanplay 服务和 JSON-RPC daemon
- `native/mpv-render-bridge/`: 可选的 libmpv 原生渲染桥
- `scripts/`: 诊断脚本和 release 资源准备脚本
- `.github/workflows/release.yml`: Windows/macOS/Linux 自动打包与发布 workflow

旧 Slint 前端已经作废，不再作为当前应用入口；Rust 后端继续保留，并通过 Electron IPC 接入。
