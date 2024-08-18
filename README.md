# SunShot

Welcome to a clean, Rust version of SunShot, a screen recorder that follows your mouse.

## Windows Setup

- npm install

Needs pkg-config, clang, ffmpeg on system:

- choco install pkgconfiglite
- choco install llvm
- Install vcpkg
- vcpkg integrate install
- vcpkg install ffmpeg[x264,gpl]

Run:

- npm run tauri dev

## VS Code Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
