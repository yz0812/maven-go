# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

这是一个基于 **Tauri v2 + Vue 3 + TypeScript + Vite** 的桌面应用项目。

**技术栈：**
- **前端：** Vue 3（使用 Composition API `<script setup>`）、TypeScript、Vite
- **后端：** Rust（Tauri v2）
- **包管理：** pnpm

**架构特点：**
- Tauri v2 使用 Capabilities 权限模型（配置文件位于 `src-tauri/capabilities/`）
- Rust 库名为 `mavengo_lib`（见 `src-tauri/Cargo.toml:14`，解决 Windows 命名冲突）
- 前端通过 `@tauri-apps/api/core` 的 `invoke` 调用 Rust Command
- Vite 开发服务器固定端口 `1420`（strictPort: true）

## 常用命令

### 开发
```bash
# 启动 Tauri 开发模式（自动启动 Vite + Rust 后端）
pnpm tauri dev
```

### 构建
```bash
# 前端类型检查 + 构建
pnpm build

# 构建桌面应用程序
pnpm tauri build
```

### 其他
```bash
# 仅启动 Vite 前端开发服务器（不启动 Tauri）
pnpm dev

# 预览生产构建
pnpm preview
```

## 核心架构说明

### 前后端通信（IPC）
- **前端调用：** 使用 `invoke("command_name", { args })` 从 `@tauri-apps/api/core`
- **后端定义：** 在 `src-tauri/src/lib.rs` 中使用 `#[tauri::command]` 宏
- **注册 Command：** 必须在 `lib.rs` 的 `tauri::Builder` 中通过 `.invoke_handler(tauri::generate_handler![command1, command2])` 注册

**示例链路：**
```typescript
// src/App.vue:10
greetMsg.value = await invoke("greet", { name: name.value });
```
↓
```rust
// src-tauri/src/lib.rs:2-5
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}
```

### Tauri 权限系统（Capabilities v2）
- 配置文件：`src-tauri/capabilities/default.json`
- 当前权限：`core:default`（核心 API）、`opener:default`（打开外部链接）
- **重要：** 新增文件系统、HTTP、Shell 等 API 需在此声明权限

### 项目结构
```
mavengo/
├── src/                     # Vue 前端源码
│   ├── App.vue             # 主应用组件
│   ├── main.ts             # Vue 应用入口
│   └── vite-env.d.ts       # Vite 类型声明
├── src-tauri/              # Rust 后端源码
│   ├── src/
│   │   ├── lib.rs          # Tauri 应用主逻辑 + Commands
│   │   └── main.rs         # 入口（调用 mavengo_lib::run）
│   ├── capabilities/       # Tauri v2 权限配置
│   ├── Cargo.toml          # Rust 依赖
│   └── tauri.conf.json     # Tauri 配置（窗口、打包、构建命令）
├── package.json            # 前端依赖 + 脚本
├── vite.config.ts          # Vite 配置（固定端口 1420）
└── tsconfig.json           # TypeScript 严格模式配置
```

## 关键配置文件

### `src-tauri/tauri.conf.json`
- **开发命令：** `beforeDevCommand: "pnpm dev"`（启动 Vite）
- **构建命令：** `beforeBuildCommand: "pnpm build"`
- **默认窗口：** 800x600

### `tsconfig.json`
- **严格模式：** `strict: true`
- **启用未使用变量检查：** `noUnusedLocals`, `noUnusedParameters`

### `Cargo.toml`
- **Windows 兼容性注意：** `lib.name = "mavengo_lib"` 避免二进制与库名冲突
- **已安装插件：** `tauri-plugin-opener`

## 开发注意事项

### Rust 端
- **禁止使用 `.expect()` 和 `.unwrap()`：** 生产代码必须使用 `Result<T, E>` 返回类型
- **Command 参数序列化：** 使用 `serde` 的 `Serialize` / `Deserialize`
- **主线程阻塞：** 耗时操作使用 `async` 或 `tauri::async_runtime::spawn`

### 前端端
- **类型安全：** 所有 `invoke` 调用必须处理 `Promise` 可能的 reject
- **Vite HMR：** 修改 Rust 代码需重启 Tauri（Vite 不监听 `src-tauri/`）
- **样式：** 当前使用传统 CSS（`<style>` 块），未引入 Tailwind CSS

### 权限管理（Tauri v2 特性）
- 新增 Tauri 插件后必须在 `capabilities/default.json` 添加权限
- 权限格式：`plugin-name:permission-set`（如 `fs:default`, `http:default`）

## 版本信息
- **Tauri:** v2（使用 Capabilities 权限模型，与 v1 不兼容）
- **Vue:** ^3.5.13（Composition API）
- **TypeScript:** ~5.6.2
- **Vite:** ^6.0.3
