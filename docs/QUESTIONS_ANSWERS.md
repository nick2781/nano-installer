# 常见问题解答

## 1. 所需空间大小如何计算？

### 当前实现：
- ✅ **自动计算**：在 `build` 时，会递归计算 `payload/` 目录的总大小（解压后）
- ✅ **加缓冲**：自动加 20% 缓冲空间
- ✅ **兜底值**：如果没有 payload 目录，使用配置中的 `install.required_space_mb` 作为兜底值

### 配置示例：
```json
{
  "install": {
    "required_space_mb": 200  // 兜底值，payload 不存在时使用
  }
}
```

### 构建输出：
```
📦 Building installer executable...
   ℹ️  Calculated payload size: 150.50 MB (uncompressed)
   ✓ Config: installer_config.json (required_space: 180 MB)
```

---

## 2. JSON 语言文件会被编译成 .pak 吗？

### 当前实现：
**✅ 会自动编译**。在 `build` 时自动将 JSON 编译为 .pak 并打包：

### 构建流程：
```bash
nano-installer build -p examples/TapTap
```

构建输出：
```
📦 Building installer executable...
   📦 Packing resources...
   ℹ️  Calculated payload size: 150.50 MB (uncompressed)
   ✓ Config: installer_config.json (required_space: 180 MB)
   ✓ Payload: payload.7z (45.20 MB compressed)
   ✓ Layouts: layouts/ (4)
   ✓ Assets: assets/ (12)
   ✓ Locales: locales/ (4 compiled to .pak)  <- 自动编译
      zh-CN.json -> zh-CN.pak
      en-US.json -> en-US.pak
      ja.json -> ja.pak
      ko.json -> ko.pak
```

### 源文件（开发时）：
```
locales/
  ├── zh-CN.json  (人类可读，方便翻译)
  ├── en-US.json
  └── ja.json
```

### 打包后（运行时）：
安装器内嵌的是 `.pak` 二进制文件：
```
资源包内：
  ├── locales/zh-CN.pak  (二进制，已加密校验)
  ├── locales/en-US.pak
  └── locales/ja.pak
```

### 手动编译（可选）
```bash
# 编译单个语言包
nano-installer langpack locales/zh-CN.json -o locales/zh-CN.pak

# 或批量编译
for f in locales/*.json; do
  nano-installer langpack $f -o ${f%.json}.pak
done
```

生成的 .pak 文件：
```
locales/
  ├── zh-CN.pak  (二进制，体积更小，加载更快)
  ├── en-US.pak
  └── ja.pak
```

### .pak 格式优势：
- ✅ 二进制格式，体积更小（约 30-50% 压缩率）
- ✅ 加载速度更快
- ✅ 包含 CRC32 校验，防止篡改
- ✅ 可选加密支持

### .pak 文件结构：
```
魔数 "LNGP" (4B)
版本号 (2B)
语言代码长度 (2B) + 语言代码 (如 "zh-CN")
CRC32 校验 (4B)
键值对数量 (4B)
键值对列表:
  - 键长度 (2B) + 键 (UTF-8)
  - 值长度 (4B) + 值 (UTF-8)
  - ... (重复)
```

---

## 3. 文本超宽怎么办？

### 问题场景：
不同语言的文本长度差异很大，例如：
- 英文：`Install` (7个字符)
- 德文：`Installieren` (13个字符)
- 中文：`安装` (2个字符)

### 解决方案：

#### 方案 A：XML 布局中不指定固定宽度
```xml
<!-- 不推荐：固定宽度 -->
<Button id="install" text="安装" text_i18n="button.install" width="80" />

<!-- 推荐：自动宽度 -->
<Button id="install" text="安装" text_i18n="button.install" min_width="80" />
```

#### 方案 B：使用响应式布局
```xml
<HBox spacing="10">
  <Button id="back" text="上一步" text_i18n="button.back" flex="1" />
  <Button id="next" text="下一步" text_i18n="button.next" flex="1" />
</HBox>
```

#### 方案 C：文本截断与省略号
在 `Label` 中支持 `overflow` 属性：
```xml
<Label text="This is a very long text" 
       text_i18n="welcome.description"
       width="200" 
       overflow="ellipsis" />
```
显示为：`This is a very lon...`

#### 方案 D：自动换行
```xml
<Label text="点击\"安装\"即表示您同意我们的服务条款和隐私政策" 
       text_i18n="welcome.agreement"
       width="400"
       wrap="true"
       max_lines="3" />
```

#### 方案 E：运行时警告
开发模式下，如果文本超出容器：
```
⚠️  Text overflow detected:
    Element: Button#install
    Language: de-DE
    Text: "Installieren" (130px)
    Container: 100px
    Suggestion: Increase width to 150px or use min_width
```

### 最佳实践：

1. **设计时预留空间**
   - 英文按钮宽度 × 1.5 = 其他语言的参考宽度
   - 对于德语、俄语等长单词语言，预留 2倍 空间

2. **使用灵活布局**
   ```xml
   <!-- 好的做法 -->
   <HBox>
     <Spacer flex="1" />
     <Button id="install" text="Install" text_i18n="button.install" />
     <Spacer width="10" />
     <Button id="cancel" text="Cancel" text_i18n="button.cancel" />
   </HBox>
   ```

3. **提供简短版本文案**
   ```json
   {
     "button.install": "安装",
     "button.install.short": "装",
     "button.install.full": "立即安装"
   }
   ```

4. **测试所有语言**
   ```bash
   # 切换语言测试
   nano-installer.exe --locale de-DE
   nano-installer.exe --locale ja-JP
   ```

5. **使用最长语言作为基准**
   - 通常是德语（de-DE）或俄语（ru-RU）
   - 设计时以最长文本为准

---

## 4. 输出文件名和图标如何配置？

### 配置示例：
```json
{
  "output": {
    "installer_name": "TapTap_Setup.exe",
    "installer_icon": "assets/logo.ico",
    "uninstaller_name": "uninst.exe",
    "uninstaller_icon": "assets/uninst.ico"
  }
}
```

### 支持的配置：
- `installer_name`: 安装器文件名（默认：`{project_name}_Setup.exe`）
- `installer_icon`: 安装器图标路径（相对于项目根目录）
- `uninstaller_name`: 卸载器文件名（默认：`uninst.exe`）
- `uninstaller_icon`: 卸载器图标路径

### 构建时的处理：
1. 读取配置中的文件名
2. 从指定路径加载 .ico 文件
3. 使用 `winres` 在编译时嵌入图标
4. 生成指定名称的可执行文件

---

## 5. payload 目录 vs payload.7z

### 两种方式：

#### 方式 1：使用 payload/ 目录（开发）
```
project/
  ├── payload/
  │   ├── TapTap.exe
  │   ├── config.json
  │   └── assets/
  │       └── ...
```
- `build` 时会自动计算目录大小
- 需要手动压缩：`7z a payload.7z payload/*`

#### 方式 2：使用 payload.7z（发布）
```
project/
  ├── payload.7z  (已压缩)
```
- 直接使用预先压缩的文件
- 构建更快

### 自动化脚本（TODO）：
```powershell
# build.ps1
if (Test-Path "payload" -PathType Container) {
    Write-Host "Compressing payload..."
    7z a -t7z -mx=9 payload.7z payload\*
}

nano-installer build
```


