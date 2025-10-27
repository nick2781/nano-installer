#!/bin/bash
# 构建脚本

set -e

echo "Building Nano Installer..."

# 1. 构建语言包
echo "Step 1: Building language packs..."
cargo run --bin langpack-builder -- locales dist/locales

# 2. 编译安装器
echo "Step 2: Building installer..."
cargo build --release --bin installer

# 3. 编译卸载器
echo "Step 3: Building uninstaller..."
cargo build --release --bin uninstaller

# 4. 复制到 dist 目录
echo "Step 4: Copying binaries to dist..."
mkdir -p dist
cp target/release/installer.exe dist/ 2>/dev/null || cp target/release/installer dist/
cp target/release/uninstaller.exe dist/ 2>/dev/null || cp target/release/uninstaller dist/

echo "Build completed!"
echo "Outputs:"
echo "  - dist/installer"
echo "  - dist/uninstaller"
echo "  - dist/locales/*.pak"

