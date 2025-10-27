# Windows 构建脚本

Write-Host "Building Nano Installer..." -ForegroundColor Green

# 1. 构建语言包
Write-Host "`nStep 1: Building language packs..." -ForegroundColor Cyan
cargo run --bin langpack-builder -- locales dist/locales

# 2. 编译安装器
Write-Host "`nStep 2: Building installer..." -ForegroundColor Cyan
cargo build --release --bin installer

# 3. 编译卸载器
Write-Host "`nStep 3: Building uninstaller..." -ForegroundColor Cyan
cargo build --release --bin uninstaller

# 4. 复制到 dist 目录
Write-Host "`nStep 4: Copying binaries to dist..." -ForegroundColor Cyan
New-Item -ItemType Directory -Force -Path dist | Out-Null
Copy-Item target/release/installer.exe dist/ -ErrorAction SilentlyContinue
Copy-Item target/release/uninstaller.exe dist/ -ErrorAction SilentlyContinue

Write-Host "`nBuild completed!" -ForegroundColor Green
Write-Host "Outputs:"
Write-Host "  - dist/installer.exe"
Write-Host "  - dist/uninstaller.exe"
Write-Host "  - dist/locales/*.pak"

