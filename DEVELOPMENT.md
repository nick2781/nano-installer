# 开发工作流程

## 完整编译流程

当修改代码后，需要按照以下顺序重新编译：

### 方法 1: 使用一键脚本（推荐）

```bash
# Windows
./rebuild-all.bat

# Linux/Mac
./rebuild-all.sh
```

### 方法 2: 手动分步编译

#### 1. 编译 nano-installer CLI（如果修改了 CLI 代码）
```bash
cargo build --bin nano-installer        # debug 版本
```

#### 2. 编译 lzma stub（如果修改了安装器运行时代码）
```bash
cargo build --bin lzma-x64-unicode              # debug 版本（带控制台输出）
cargo build --release --bin lzma-x64-unicode    # release 版本（生产环境）
```

#### 3. 编译 uninst stub（如果修改了卸载器代码）
```bash
cargo build --release --bin uninst
```

#### 4. 构建测试项目
```bash
cd examples/TapTap
../../target/debug/nano-installer.exe build
```

## 构建器优先级

nano-installer 构建器会按以下优先级选择 stub：

**安装器 stub (lzma-x64-unicode.exe):**
1. `target/debug/lzma-x64-unicode.exe` ← 最优先（带控制台输出，方便调试）
2. `../../target/debug/lzma-x64-unicode.exe`
3. `target/release/lzma-x64-unicode.exe`
4. `../../target/release/lzma-x64-unicode.exe`

**卸载器 stub (uninst.exe):**
1. `target/release/uninst.exe` ← 最优先
2. `../../target/release/uninst.exe`

## 测试流程

### Debug 版本测试（推荐）
```bash
cd examples/TapTap
./dist/TapTap_Setup.exe
```
- 会显示控制台窗口和调试输出
- 便于查看错误信息
- 日志输出到控制台

### Release 版本测试
```bash
# 临时移开 debug 版本
mv target/debug/lzma-x64-unicode.exe target/debug/lzma-x64-unicode.exe.bak

# 重新构建（会使用 release stub）
cd examples/TapTap
../../target/debug/nano-installer.exe build

# 测试
./dist/TapTap_Setup.exe

# 恢复 debug 版本
mv target/debug/lzma-x64-unicode.exe.bak target/debug/lzma-x64-unicode.exe
```

## 常见问题

### Q: 修改代码后没有生效？
**A:** 检查是否完整编译了所有组件：
1. 检查修改的是哪个组件（CLI、lib、stub）
2. 重新编译对应的组件
3. 重新构建测试项目

### Q: 图片或资源没有加载？
**A:** 确保：
1. 资源文件存在于 `examples/TapTap/assets/` 或 `examples/TapTap/layouts/`
2. 重新运行 `nano-installer build`
3. 检查控制台/日志中的资源加载错误

### Q: 中文显示为方框？
**A:** 确保：
1. 已添加中文字体加载代码
2. 重新编译了 stub（debug 或 release）
3. 重新构建了测试项目

### Q: XML 解析失败？
**A:** 检查：
1. XML 格式是否正确（UTF-8 编码）
2. 特殊字符是否正确转义（`"` → `&quot;`, `<` → `&lt;`, 等）
3. 元素类型和属性是否支持

## 文件大小参考

- **nano-installer CLI**: ~600 KB (debug)
- **lzma-x64-unicode stub**: ~15 MB (debug), ~4 MB (release)
- **uninst stub**: ~200 KB (release)
- **TapTap_Setup.exe**: ~7-18 MB（取决于使用的 stub 版本和资源大小）

## 快速命令参考

```bash
# 完整重建
./rebuild-all.bat

# 仅编译 stub
cargo build --release --bin lzma-x64-unicode

# 仅构建安装器
cd examples/TapTap && ../../target/debug/nano-installer.exe build

# 查看日志（Windows）
notepad %TEMP%\install_*.log
```
