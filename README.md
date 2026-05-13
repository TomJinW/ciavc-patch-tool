# 3ds 宝可梦 VC ROM 替换与补丁工具

## 如何搭建环境：

https://v2.tauri.app/start/prerequisites/

## 关于本 App CLI 的使用方法

```
ciavc-patch-tool input/cia/path.cia output/cia/path.cia [-r input/rom/path.gbc] [-p input/patch/path.patch] [-g] [-k]
```
- --rom (-r) 要替换的  ROM 文件
- --patch (-g) 要替换的 patch 文件
- --gen2patch (-g) 启用 Gen 2 VC 房间码修改
- --iskorean (-k) 表示输入的原始 cia 是韩版

## Tauri + Vue + TypeScript

This template should help get you started developing with Vue 3 and TypeScript in Vite. The template uses Vue 3 `<script setup>` SFCs, check out the [script setup docs](https://v3.vuejs.org/api/sfc-script-setup.html#sfc-script-setup) to learn more.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
