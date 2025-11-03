# XML 布局指南

nano-installer 使用 XML 来定义安装程序的用户界面。这份指南将教你如何创建和自定义界面布局。

## 📖 目录

- [基本结构](#基本结构)
- [布局元素](#布局元素)
- [元素属性](#元素属性)
- [变量替换](#变量替换)
- [DPI 支持](#dpi-支持)
- [完整示例](#完整示例)

## 基本结构

每个 XML 布局文件必须包含以下基本结构：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Layout name="PageName" version="1.0.0">
  <Page>
    <!-- 页面内容 -->
  </Page>
</Layout>
```

### 根元素属性

| 属性 | 必需 | 说明 | 示例 |
|------|------|------|------|
| `name` | 是 | 布局名称 | `"Welcome"` |
| `version` | 是 | 布局版本 | `"1.0.0"` |

## 布局元素

### 容器元素

#### `<VBox>` - 垂直布局

将子元素垂直排列。

```xml
<VBox>
  <Label text="第一行" />
  <Label text="第二行" />
  <Label text="第三行" />
</VBox>
```

**属性：**
- `padding` - 内边距，格式：`"top,right,bottom,left"`
- `spacing` - 子元素间距（像素）
- `alignment` - 对齐方式：`"left"` | `"center"` | `"right"`

#### `<HBox>` - 水平布局

将子元素水平排列。

```xml
<HBox>
  <Button text="按钮1" />
  <Button text="按钮2" />
  <Button text="按钮3" />
</HBox>
```

**属性：**
- `padding` - 内边距
- `spacing` - 子元素间距
- `alignment` - 对齐方式：`"top"` | `"center"` | `"bottom"`

### 基础元素

#### `<Label>` - 文本标签

显示文本内容。

```xml
<Label 
  text="欢迎使用 {product_name}" 
  font_size="24" 
  color="#FFFFFF"
/>
```

**属性：**

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `text` | string | 必需 | 显示的文本内容 |
| `font_size` | number | 14 | 字体大小（像素） |
| `color` | string | `"#FFFFFF"` | 文本颜色（十六进制） |
| `bold` | boolean | false | 是否加粗 |
| `italic` | boolean | false | 是否斜体 |
| `alignment` | string | `"left"` | 对齐：`"left"` \| `"center"` \| `"right"` |

#### `<Button>` - 按钮

可点击的按钮。

```xml
<Button 
  id="next" 
  text="下一步" 
  width="120" 
  height="40" 
  style="primary"
/>
```

**属性：**

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `id` | string | 必需 | 按钮标识符，用于事件处理 |
| `text` | string | 必需 | 按钮文字 |
| `width` | number | 120 | 按钮宽度（像素） |
| `height` | number | 40 | 按钮高度（像素） |
| `enabled` | boolean | true | 是否启用 |
| `style` | string | `"default"` | 样式：`"primary"` \| `"secondary"` \| `"link"` |

**按钮 ID 约定：**

| ID | 功能 |
|-----|------|
| `next` | 下一步按钮 |
| `back` | 上一步按钮 |
| `cancel` | 取消按钮 |
| `install` | 开始安装按钮 |
| `finish` | 完成按钮 |
| `browse` | 浏览文件夹按钮 |

#### `<Image>` - 图片

显示图片资源。

```xml
<Image 
  icon="assets/logo.png" 
  width="200" 
  height="60" 
/>
```

**属性：**

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `icon` | string | 必需 | 图片路径（相对于项目根目录） |
| `width` | number | 可选 | 图片宽度，不指定则使用原始尺寸 |
| `height` | number | 可选 | 图片高度 |

**注意：** 系统会自动根据 DPI 加载 `@2x` 版本的图片。

#### `<Checkbox>` - 复选框

用户可勾选的选项。

```xml
<Checkbox 
  id="agree_license" 
  text="我同意许可协议" 
  checked="false"
/>
```

**属性：**

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `id` | string | 必需 | 复选框标识符 |
| `text` | string | 必需 | 显示文本 |
| `checked` | boolean | false | 初始状态 |
| `enabled` | boolean | true | 是否启用 |

**常用 ID：**

| ID | 功能 |
|-----|------|
| `desktop_shortcut` | 创建桌面快捷方式 |
| `start_menu` | 添加到开始菜单 |
| `auto_start` | 开机自动启动 |
| `agree_license` | 同意许可协议 |

#### `<TextInput>` - 文本输入框

用户输入文本的控件。

```xml
<TextInput 
  id="install_path" 
  text="C:\Program Files\MyApp" 
  width="300"
/>
```

**属性：**

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `id` | string | 必需 | 输入框标识符 |
| `text` | string | `""` | 初始文本 |
| `placeholder` | string | `""` | 占位符文本 |
| `width` | number | 200 | 宽度（像素） |
| `readonly` | boolean | false | 是否只读 |

#### `<ProgressBar>` - 进度条

显示安装进度。

```xml
<ProgressBar 
  id="install_progress" 
  progress="0.5" 
  width="400" 
  height="8"
/>
```

**属性：**

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `id` | string | 必需 | 进度条标识符 |
| `progress` | number | 0.0 | 进度值（0.0 ~ 1.0） |
| `width` | number | 300 | 宽度（像素） |
| `height` | number | 8 | 高度（像素） |

#### `<Spacer>` - 间距

创建空白间距。

```xml
<!-- 垂直间距 -->
<Spacer height="20" />

<!-- 水平间距 -->
<Spacer width="10" />
```

**属性：**

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `width` | number | 0 | 水平间距 |
| `height` | number | 0 | 垂直间距 |

## 元素属性

### 通用属性

所有元素都支持以下属性：

| 属性 | 类型 | 说明 |
|------|------|------|
| `id` | string | 元素唯一标识符 |
| `visible` | boolean | 是否可见（默认 true） |
| `enabled` | boolean | 是否启用（默认 true） |

### 尺寸属性

| 属性 | 类型 | 说明 |
|------|------|------|
| `width` | number | 宽度（像素） |
| `height` | number | 高度（像素） |
| `min_width` | number | 最小宽度 |
| `min_height` | number | 最小高度 |
| `max_width` | number | 最大宽度 |
| `max_height` | number | 最大高度 |

### 样式属性

| 属性 | 类型 | 说明 |
|------|------|------|
| `color` | string | 颜色（十六进制，如 `"#FF0000"`) |
| `background` | string | 背景色 |
| `font_size` | number | 字体大小 |
| `bold` | boolean | 是否加粗 |
| `italic` | boolean | 是否斜体 |

## 变量替换

在文本中使用 `{variable}` 语法可以插入动态内容：

```xml
<Label text="欢迎使用 {product_name}" />
<Label text="版本：{version}" />
<Label text="发布者：{publisher}" />
```

### 可用变量

| 变量 | 来源 | 说明 |
|------|------|------|
| `{product_name}` | `project.name` | 产品名称 |
| `{version}` | `project.version` | 版本号 |
| `{publisher}` | `project.publisher` | 发布者 |
| `{install_path}` | 用户输入 | 安装路径 |
| `{required_space}` | `install.required_space_mb` | 所需空间 |

### 语言字符串

使用 `{key}` 引用 `locales/*.json` 中定义的字符串：

```xml
<Label text="{welcome_message}" />
<Button text="{install_button}" />
```

对应的 `locales/zh-CN.json`：

```json
{
  "language_name": "简体中文",
  "strings": {
    "welcome_message": "欢迎使用本软件",
    "install_button": "立即安装"
  }
}
```

## DPI 支持

nano-installer 自动支持高 DPI 屏幕。

### 资源文件命名

为图片提供 2x 版本：

```
assets/
├── logo.png        # 1x 版本（例如 100x30 像素）
└── logo@2x.png     # 2x 版本（200x60 像素）
```

系统会根据屏幕 DPI 自动选择合适的版本。

### 布局文件命名

同样可以为不同 DPI 提供不同的布局：

```
layouts/
├── welcome.xml     # 标准 DPI
└── welcome@2x.xml  # 高 DPI
```

**注意：** 大多数情况下不需要 `@2x.xml`，只需提供 `@2x` 图片资源即可。

## 完整示例

### 欢迎页面

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Layout name="Welcome" version="1.0.0">
  <Page>
    <VBox padding="40,40,40,40" spacing="10">
      <!-- Logo -->
      <Image icon="assets/logo.png" width="200" height="60" />
      <Spacer height="30" />
      
      <!-- 标题 -->
      <Label 
        text="欢迎使用 {product_name}" 
        font_size="28" 
        color="#FFFFFF"
        bold="true"
        alignment="center"
      />
      
      <!-- 版本信息 -->
      <Label 
        text="版本 {version}" 
        font_size="14" 
        color="#AAAAAA"
        alignment="center"
      />
      
      <Spacer height="20" />
      
      <!-- 描述 -->
      <Label 
        text="{welcome_description}" 
        font_size="14" 
        color="#CCCCCC"
        alignment="center"
      />
      
      <Spacer height="40" />
      
      <!-- 按钮 -->
      <HBox spacing="15" alignment="center">
        <Button 
          id="next" 
          text="{next_button}" 
          width="140" 
          height="44"
          style="primary"
        />
        <Button 
          id="cancel" 
          text="{cancel_button}" 
          width="140" 
          height="44"
        />
      </HBox>
    </VBox>
  </Page>
</Layout>
```

### 安装路径配置页面

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Layout name="InstallPath" version="1.0.0">
  <Page>
    <VBox padding="40,40,40,40" spacing="15">
      <!-- 标题 -->
      <Label 
        text="{install_location_title}" 
        font_size="20" 
        bold="true"
      />
      
      <Spacer height="10" />
      
      <!-- 说明 -->
      <Label 
        text="{install_location_desc}" 
        font_size="12" 
        color="#AAAAAA"
      />
      
      <Spacer height="20" />
      
      <!-- 安装路径 -->
      <HBox spacing="10">
        <TextInput 
          id="install_path" 
          text="{install_path}" 
          width="400"
        />
        <Button 
          id="browse" 
          text="{browse_button}" 
          width="80"
        />
      </HBox>
      
      <Spacer height="10" />
      
      <!-- 空间信息 -->
      <Label 
        text="{required_space_label}: {required_space} MB" 
        font_size="12" 
        color="#888888"
      />
      
      <Spacer height="30" />
      
      <!-- 选项 -->
      <Checkbox 
        id="desktop_shortcut" 
        text="{create_desktop_shortcut}" 
        checked="true"
      />
      <Checkbox 
        id="start_menu" 
        text="{create_start_menu}" 
        checked="true"
      />
      <Checkbox 
        id="auto_start" 
        text="{auto_start_option}" 
        checked="false"
      />
      
      <Spacer height="40" />
      
      <!-- 按钮 -->
      <HBox spacing="15">
        <Button id="back" text="{back_button}" width="120" />
        <Button id="install" text="{install_button}" width="120" style="primary" />
        <Spacer width="auto" />
        <Button id="cancel" text="{cancel_button}" width="120" />
      </HBox>
    </VBox>
  </Page>
</Layout>
```

### 安装进度页面

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Layout name="Installing" version="1.0.0">
  <Page>
    <VBox padding="40,40,40,40" spacing="20" alignment="center">
      <!-- 标题 -->
      <Label 
        text="{installing_title}" 
        font_size="24" 
        bold="true"
      />
      
      <Spacer height="30" />
      
      <!-- 进度条 -->
      <ProgressBar 
        id="install_progress" 
        progress="0.0" 
        width="500" 
        height="12"
      />
      
      <Spacer height="15" />
      
      <!-- 当前状态 -->
      <Label 
        id="status_text" 
        text="{status_extracting}" 
        font_size="14" 
        color="#AAAAAA"
      />
      
      <Spacer height="50" />
      
      <!-- 取消按钮 -->
      <Button 
        id="cancel" 
        text="{cancel_button}" 
        width="120" 
        enabled="false"
      />
    </VBox>
  </Page>
</Layout>
```

## 调试技巧

### 1. 使用调试模式

在配置文件中启用调试：

```json
{
  "advanced": {
    "debug_mode": true
  }
}
```

### 2. 验证 XML

确保 XML 格式正确：
- 所有标签都正确闭合
- 属性值使用引号
- 特殊字符需要转义（`&lt;` `&gt;` `&amp;` `&quot;`）

### 3. 检查日志

运行安装程序后，查看日志文件：
```
%TEMP%\nano-installer-XXXX\install.log
```

## 最佳实践

1. **保持简洁** - 每个页面只放必要的元素
2. **使用间距** - 适当使用 `<Spacer>` 让界面更舒适
3. **一致性** - 保持按钮尺寸、字体大小一致
4. **响应式** - 考虑不同分辨率下的显示效果
5. **多语言** - 所有文本都使用变量，不要硬编码
6. **高DPI** - 为所有图片提供 @2x 版本

## 相关文档

- [配置参考](CONFIG_REFERENCE.md) - 完整的配置选项说明
- [多语言支持](LOCALIZATION.md) - 如何添加语言
- [示例项目](../examples/TapTap/README.md) - 查看完整示例

---

有问题？查看 [examples/TapTap/layouts/](../examples/TapTap/layouts/) 中的实际示例！
