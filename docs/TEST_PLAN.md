# nano-installer 功能测试计划

## 测试环境

- OS: Windows 10/11
- DPI: 标准 (96) 和高 DPI (144+)
- Rust: stable-x86_64-pc-windows-msvc

## 测试分层

- `library harness`
  - 优先验证布局矩形、语言切换、动作分发、弹窗状态
  - 入口见 [test_harness.rs](/D:/taptap-pc/nano-installer/installer/lib/src/ui/test_harness.rs)
  - CLI 诊断入口：`nano-installer harness snapshot --project <dir> ...`
- `script smoke`
  - 验证真实 EXE 主链路
  - 入口见 [smoke_test.ps1](/D:/taptap-pc/nano-installer/scripts/smoke_test.ps1)、
    [gui_real_test.ps1](/D:/taptap-pc/nano-installer/scripts/gui_real_test.ps1)
- `manual review`
  - 只做最终视觉验收

## 测试项目：examples/TapTap

### 当前状态检查

- [x] `nano-installer.exe` 已编译（带图标）
- [ ] TapTap 项目完整性
- [ ] 所有资源文件存在

## 测试计划

### 阶段 1：项目结构验证

1. **检查 TapTap 项目文件**
   ```powershell
   cd examples\TapTap
   ls
   ```
   
   期望文件：
   - ✓ `installer_config.json`
   - ✓ `assets/` (包含图片和图标)
   - ✓ `layouts/` (包含 XML 布局)
   - ✓ `locales/` (包含语言文件)
   - ✓ `files/` (待安装文件目录)
   - ✓ `build.ps1`
   - ✓ `run_installer.ps1`

2. **验证配置文件**
   ```powershell
   cd D:\taptap-pc\nano-installer
   .\target\release\nano-installer.exe validate examples\TapTap\installer_config.json
   ```
   
   期望结果：
   - ✓ 配置验证通过
   - 显示项目名称、版本等信息

### 阶段 2：CLI 工具测试

#### 2.1 Init 命令
```powershell
cd D:\taptap-pc\nano-installer
.\target\release\nano-installer.exe init TestProject
```

期望结果：
- 创建 `TestProject/` 目录
- 包含完整的项目结构
- 生成默认配置文件

#### 2.2 Validate 命令
```powershell
.\target\release\nano-installer.exe validate TestProject\installer_config.json
```

期望结果：
- 配置验证通过

#### 2.3 Langpack 命令
```powershell
.\target\release\nano-installer.exe langpack examples\TapTap\locales\zh-CN.json -o test.pak
```

期望结果：
- 生成 `test.pak` 文件
- 文件大小 > 0

### 阶段 3：TapTap 安装器构建

#### 3.1 准备测试文件
```powershell
cd examples\TapTap\files
# 创建测试文件
echo "test" > test.txt
```

#### 3.2 运行构建脚本
```powershell
cd examples\TapTap
.\build.ps1
```

期望结果：
- 复制 `installer.exe` → `dist/TapTap_Setup.exe`
- 创建 `payload.7z` (从 files/)
- 复制配置文件

**检查点：**
- [ ] `dist/TapTap_Setup.exe` 存在
- [ ] `dist/TapTap_Setup.exe` 有图标
- [ ] `payload.7z` 存在且大小 > 0

### 阶段 4：安装器运行测试

#### 4.1 GUI 模式启动
```powershell
cd examples\TapTap\dist
.\TapTap_Setup.exe
```

期望行为：
- [ ] 窗口正常打开
- [ ] 显示 TapTap Logo
- [ ] 显示欢迎页面
- [ ] UI 元素正确渲染

#### 4.2 布局测试

**欢迎页面：**
- [ ] Logo 图片显示
- [ ] 标题文字显示
- [ ] "下一步" 按钮可点击
- [ ] "取消" 按钮可点击

**配置页面：**
- [ ] 安装路径输入框显示
- [ ] 浏览按钮可点击
- [ ] 复选框（桌面快捷方式）可勾选
- [ ] 复选框（开始菜单）可勾选
- [ ] 复选框（自启动）可勾选

**安装页面：**
- [ ] 进度条显示
- [ ] 状态文字更新
- [ ] 可以取消安装

**完成页面：**
- [ ] 完成提示显示
- [ ] "完成" 按钮可点击
- [ ] "立即启动" 复选框可勾选

#### 4.3 语言切换测试

- [ ] 语言选择下拉框显示
- [ ] 可切换语言（zh-CN ↔ en-US）
- [ ] 界面文字随语言变化

#### 4.4 DPI 测试

**标准 DPI (96-120):**
- [ ] UI 尺寸正常
- [ ] 图片清晰
- [ ] 文字可读

**高 DPI (144-192):**
- [ ] 自动加载 @2x 图片
- [ ] UI 不模糊
- [ ] 布局正确

#### 4.5 功能测试

**路径验证：**
- [ ] 输入非法路径时提示错误
- [ ] 磁盘空间不足时提示
- [ ] 路径包含中文时正常

**快捷方式：**
- [ ] 桌面快捷方式正确创建
- [ ] 开始菜单项正确创建

**自启动：**
- [ ] 注册表项正确写入
- [ ] 重启后自动启动

**安装过程：**
- [ ] 解压 payload.7z
- [ ] 文件正确复制到安装目录
- [ ] 注册表信息写入
- [ ] 卸载程序创建

#### 4.6 命令行参数测试

**静默安装：**
```powershell
.\TapTap_Setup.exe --silent --install-path "D:\Test\TapTap"
```
- [ ] 不显示 UI
- [ ] 安装到指定目录
- [ ] 返回正确的退出代码

**指定配置：**
```powershell
.\TapTap_Setup.exe --config ..\installer_config.json
```
- [ ] 从指定路径加载配置

### 阶段 5：卸载测试

#### 5.1 卸载器启动
```powershell
# 从开始菜单或控制面板启动卸载
# 或直接运行
D:\TapTap\uninst.exe
```

期望行为：
- [ ] 卸载器正常启动
- [ ] 显示卸载确认界面
- [ ] 可选择是否保留用户数据

#### 5.2 卸载功能
- [ ] 删除应用程序文件
- [ ] 删除桌面快捷方式
- [ ] 删除开始菜单项
- [ ] 删除注册表项
- [ ] （可选）保留用户数据

### 阶段 6：错误处理测试

#### 6.1 缺少资源文件
```powershell
# 删除某个资源文件测试
rm examples\TapTap\assets\logo.png
.\TapTap_Setup.exe
```
- [ ] 显示友好的错误提示
- [ ] 不崩溃

#### 6.2 配置文件错误
```powershell
# 修改配置为无效 JSON
.\TapTap_Setup.exe
```
- [ ] 显示配置解析错误
- [ ] 提示用户检查配置

#### 6.3 磁盘空间不足
- [ ] 检测磁盘空间
- [ ] 空间不足时提示
- [ ] 阻止安装

#### 6.4 权限不足
```powershell
# 尝试安装到 C:\Program Files (需要管理员权限)
.\TapTap_Setup.exe
```
- [ ] 检测权限要求
- [ ] 提示提升权限或更改路径

### 阶段 7：性能测试

#### 7.1 启动时间
- [ ] 安装器启动 < 2秒
- [ ] UI 响应流畅

#### 7.2 安装速度
- [ ] 解压 100MB payload < 10秒
- [ ] 进度条实时更新

#### 7.3 资源占用
- [ ] 内存占用 < 100MB
- [ ] CPU 占用合理

## 已知问题记录

### 问题 1: [描述]
- **现象**: 
- **复现步骤**: 
- **预期行为**: 
- **实际行为**: 
- **状态**: 

## 测试结果汇总

### 通过的测试
- [ ] CLI 工具功能
- [ ] 项目初始化
- [ ] 配置验证
- [ ] 语言包构建
- [ ] 安装器构建
- [ ] GUI 显示
- [ ] 多语言支持
- [ ] DPI 支持
- [ ] 安装功能
- [ ] 卸载功能
- [ ] 错误处理

### 失败的测试
(待记录)

### 需要修复的问题
(待记录)

## 下一步

1. 按顺序执行测试计划
2. 记录所有问题
3. 优先修复阻塞性问题
4. 完善文档

## 测试命令快速参考

```powershell
# 编译 nano-installer
cargo build --release --bin nano-installer

# 验证配置
.\target\release\nano-installer.exe validate examples\TapTap\installer_config.json

# 初始化新项目
.\target\release\nano-installer.exe init NewProject

# 构建 TapTap 安装器
cd examples\TapTap
.\build.ps1

# 运行安装器
.\dist\TapTap_Setup.exe

# 静默安装
.\dist\TapTap_Setup.exe --silent --install-path "D:\TapTap"
```

