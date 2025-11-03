# XML 布局规范

本文档定义 nano-installer 的 XML 布局文件的完整规范，包括所有元素、属性的类型和约束。

## 📖 目录

- [文件结构](#文件结构)
- [根元素](#根元素)
- [容器元素](#容器元素)
- [控件元素](#控件元素)
- [属性类型](#属性类型)
- [验证规则](#验证规则)

## 文件结构

### 基本格式

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Layout name="PageName" version="1.0.0">
  <Page>
    <!-- 页面内容 -->
  </Page>
</Layout>
```

### 文件编码

- **必须使用** UTF-8 编码
- **必须包含** XML 声明
- **推荐缩进** 2 个空格

## 根元素

### `<Layout>`

布局文件的根元素。

**属性：**

| 属性 | 类型 | 必需 | 约束 | 说明 |
|------|------|------|------|------|
| `name` | string | ✅ | 非空，字母开头 | 布局名称，如 "Welcome" |
| `version` | string | ✅ | 符合语义化版本 | 版本号，如 "1.0.0" |

**子元素：**
- `<Page>` (必需，仅一个)

**示例：**

```xml
<Layout name="Welcome" version="1.0.0">
  <Page>
    ...
  </Page>
</Layout>
```

### `<Page>`

页面容器，是所有内容的父元素。

**属性：**

| 属性 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `id` | string | ❌ | - | 页面标识符 |
| `width` | number | ❌ | 从 UI 配置 | 页面宽度（像素） |
| `height` | number | ❌ | 从 UI 配置 | 页面高度（像素） |
| `background` | color | ❌ | - | 背景颜色 |

**子元素：**
- 任意布局元素（VBox, HBox, Button, Label 等）

**示例：**

```xml
<Page id="welcome" width="574" height="358" background="#1A1D28">
  <VBox>
    ...
  </VBox>
</Page>
```

## 容器元素

### `<VBox>` - 垂直布局

将子元素垂直排列。

**属性：**

| 属性 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `id` | string | ❌ | - | 元素标识符 |
| `width` | number | ❌ | auto | 容器宽度 |
| `height` | number | ❌ | auto | 容器高度 |
| `padding` | padding | ❌ | "0,0,0,0" | 内边距（上,右,下,左） |
| `spacing` | number | ❌ | 0 | 子元素间距 |
| `align` | align | ❌ | "left" | 水平对齐："left" \| "center" \| "right" |
| `background` | color | ❌ | - | 背景颜色 |
| `visible` | boolean | ❌ | true | 是否可见 |

**子元素：**
- 任意元素（无限制）

**示例：**

```xml
<VBox padding="20,20,20,20" spacing="15" align="center">
  <Label text="标题" />
  <Label text="内容" />
  <Button text="确定" />
</VBox>
```

### `<HBox>` - 水平布局

将子元素水平排列。

**属性：**

| 属性 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `id` | string | ❌ | - | 元素标识符 |
| `width` | number | ❌ | auto | 容器宽度 |
| `height` | number | ❌ | auto | 容器高度 |
| `padding` | padding | ❌ | "0,0,0,0" | 内边距 |
| `spacing` | number | ❌ | 0 | 子元素间距 |
| `align` | align | ❌ | "top" | 垂直对齐："top" \| "center" \| "bottom" |
| `background` | color | ❌ | - | 背景颜色 |
| `visible` | boolean | ❌ | true | 是否可见 |

**子元素：**
- 任意元素（无限制）

**示例：**

```xml
<HBox spacing="10" align="center">
  <Button text="取消" width="100" />
  <Button text="确定" width="100" />
</HBox>
```

### `<Spacer>` - 间距

创建固定或弹性的空白间距。

**属性：**

| 属性 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `width` | number \| "auto" | ❌ | 0 | 水平间距，"auto" 表示占满剩余空间 |
| `height` | number \| "auto" | ❌ | 0 | 垂直间距，"auto" 表示占满剩余空间 |

**子元素：**
- 无

**示例：**

```xml
<!-- 固定间距 -->
<Spacer height="20" />

<!-- 弹性间距（填充剩余空间） -->
<Spacer width="auto" />
```

## 控件元素

### `<Button>` - 按钮

可点击的按钮控件。

**属性：**

| 属性 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `id` | string | ✅ | - | 按钮标识符，用于事件处理 |
| `text` | string | ✅ | - | 按钮文本，支持 {key} 语法 |
| `width` | number | ❌ | 120 | 按钮宽度 |
| `height` | number | ❌ | 40 | 按钮高度 |
| `style` | button-style | ❌ | "default" | 样式类型 |
| `enabled` | boolean | ❌ | true | 是否启用 |
| `visible` | boolean | ❌ | true | 是否可见 |

**button-style 枚举：**
- `"primary"` - 主要按钮
- `"secondary"` - 次要按钮
- `"link"` - 链接样式按钮
- `"text"` - 纯文本按钮

**预定义 ID：**

| ID | 功能 | 说明 |
|-----|------|------|
| `next` | 下一步 | 进入下一页 |
| `back` | 上一步 | 返回上一页 |
| `cancel` | 取消 | 取消安装 |
| `install` | 开始安装 | 开始安装过程 |
| `finish` | 完成 | 完成安装并关闭 |
| `browse` | 浏览 | 打开文件夹选择对话框 |

**子元素：**
- 无

**示例：**

```xml
<Button id="next" text="{next_button}" width="120" height="44" style="primary" />
<Button id="cancel" text="{cancel_button}" width="120" />
```

### `<Label>` - 文本标签

显示文本内容。

**属性：**

| 属性 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `id` | string | ❌ | - | 标签标识符 |
| `text` | string | ✅ | - | 显示文本，支持 {key} 语法和变量 |
| `font_size` | number | ❌ | 14 | 字体大小（像素） |
| `color` | color | ❌ | "#FFFFFF" | 文字颜色 |
| `bold` | boolean | ❌ | false | 是否加粗 |
| `italic` | boolean | ❌ | false | 是否斜体 |
| `underline` | boolean | ❌ | false | 是否下划线 |
| `align` | text-align | ❌ | "left" | 对齐方式 |
| `width` | number | ❌ | auto | 宽度限制 |
| `wrap` | boolean | ❌ | true | 是否自动换行 |
| `visible` | boolean | ❌ | true | 是否可见 |

**text-align 枚举：**
- `"left"` - 左对齐
- `"center"` - 居中
- `"right"` - 右对齐

**子元素：**
- 无

**示例：**

```xml
<Label text="欢迎使用 {product_name}" font_size="24" color="#FFFFFF" bold="true" align="center" />
<Label text="{welcome_message}" font_size="14" color="#AAAAAA" />
```

### `<Image>` - 图片

显示图片资源。

**属性：**

| 属性 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `id` | string | ❌ | - | 图片标识符 |
| `icon` | path | ✅ | - | 图片路径（相对于项目根目录） |
| `width` | number | ❌ | 原始宽度 | 显示宽度 |
| `height` | number | ❌ | 原始高度 | 显示高度 |
| `visible` | boolean | ❌ | true | 是否可见 |

**图片路径：**
- 相对路径：`"assets/logo.png"`
- 自动 DPI 适配：系统会查找 `@2x` 版本

**子元素：**
- 无

**示例：**

```xml
<Image icon="assets/logo.png" width="200" height="60" />
```

### `<Checkbox>` - 复选框

用户可勾选的选项。

**属性：**

| 属性 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `id` | string | ✅ | - | 复选框标识符 |
| `text` | string | ✅ | - | 显示文本，支持 {key} 语法 |
| `checked` | boolean | ❌ | false | 初始勾选状态 |
| `enabled` | boolean | ❌ | true | 是否启用 |
| `visible` | boolean | ❌ | true | 是否可见 |

**预定义 ID：**

| ID | 功能 | 说明 |
|-----|------|------|
| `desktop_shortcut` | 桌面快捷方式 | 创建桌面快捷方式 |
| `start_menu` | 开始菜单 | 添加到开始菜单 |
| `auto_start` | 开机自启 | 开机自动启动 |
| `agree_license` | 同意协议 | 同意许可协议 |

**子元素：**
- 无

**示例：**

```xml
<Checkbox id="desktop_shortcut" text="{create_desktop_shortcut}" checked="true" />
<Checkbox id="auto_start" text="{auto_start_option}" checked="false" />
```

### `<TextInput>` - 文本输入框

用户输入文本的控件。

**属性：**

| 属性 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `id` | string | ✅ | - | 输入框标识符 |
| `text` | string | ❌ | "" | 初始文本内容 |
| `placeholder` | string | ❌ | "" | 占位符文本 |
| `width` | number | ❌ | 200 | 输入框宽度 |
| `height` | number | ❌ | 32 | 输入框高度 |
| `readonly` | boolean | ❌ | false | 是否只读 |
| `password` | boolean | ❌ | false | 是否密码框（显示为 ***） |
| `multiline` | boolean | ❌ | false | 是否多行 |
| `enabled` | boolean | ❌ | true | 是否启用 |
| `visible` | boolean | ❌ | true | 是否可见 |

**预定义 ID：**

| ID | 功能 | 说明 |
|-----|------|------|
| `install_path` | 安装路径 | 显示和编辑安装路径 |

**子元素：**
- 无

**示例：**

```xml
<TextInput id="install_path" text="{install_path}" width="400" readonly="true" />
<TextInput id="username" placeholder="请输入用户名" width="300" />
```

### `<ProgressBar>` - 进度条

显示进度的控件。

**属性：**

| 属性 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `id` | string | ✅ | - | 进度条标识符 |
| `progress` | number | ❌ | 0.0 | 进度值（0.0 ~ 1.0） |
| `width` | number | ❌ | 300 | 进度条宽度 |
| `height` | number | ❌ | 8 | 进度条高度 |
| `style` | progress-style | ❌ | "default" | 样式类型 |
| `visible` | boolean | ❌ | true | 是否可见 |

**progress-style 枚举：**
- `"default"` - 默认样式
- `"smooth"` - 平滑动画
- `"striped"` - 条纹样式

**预定义 ID：**

| ID | 功能 | 说明 |
|-----|------|------|
| `install_progress` | 安装进度 | 显示安装进度 |
| `download_progress` | 下载进度 | 显示下载进度 |

**子元素：**
- 无

**示例：**

```xml
<ProgressBar id="install_progress" progress="0.0" width="500" height="12" />
```

### `<Divider>` - 分隔线

视觉分隔线。

**属性：**

| 属性 | 类型 | 必需 | 默认值 | 说明 |
|------|------|------|--------|------|
| `id` | string | ❌ | - | 分隔线标识符 |
| `width` | number \| "auto" | ❌ | "auto" | 宽度 |
| `height` | number | ❌ | 1 | 高度（厚度） |
| `color` | color | ❌ | "#333333" | 颜色 |
| `visible` | boolean | ❌ | true | 是否可见 |

**子元素：**
- 无

**示例：**

```xml
<Divider width="auto" height="1" color="#444444" />
```

## 属性类型

### 基本类型

#### string
任意文本字符串。

**示例：**
```xml
text="欢迎使用"
id="welcome_button"
```

#### number
数字值（整数或小数）。

**示例：**
```xml
width="574"
height="358"
progress="0.5"
font_size="14"
```

#### boolean
布尔值：`true` 或 `false`。

**示例：**
```xml
enabled="true"
visible="false"
bold="true"
```

### 复合类型

#### color
颜色值，支持以下格式：

- **十六进制**: `"#RRGGBB"` 或 `"#RRGGBBAA"`
- **命名颜色**: `"red"`, `"blue"`, `"white"` 等

**示例：**
```xml
color="#FFFFFF"
background="#1A1D28"
color="#FF0000AA"
```

#### padding
内边距，格式：`"top,right,bottom,left"`

**示例：**
```xml
padding="20,20,20,20"   <!-- 四周 20px -->
padding="10,20,10,20"   <!-- 上下 10px，左右 20px -->
padding="0,0,0,0"       <!-- 无内边距 -->
```

#### path
文件路径，相对于项目根目录。

**示例：**
```xml
icon="assets/logo.png"
icon="assets/icons/button.png"
```

### 特殊值

#### auto
表示自动计算的值。

**用于：**
- `width="auto"` - 自动宽度
- `height="auto"` - 自动高度

#### 变量替换
使用 `{variable}` 语法引用变量。

**系统变量：**
- `{product_name}` - 产品名称
- `{version}` - 版本号
- `{publisher}` - 发布者
- `{install_path}` - 安装路径
- `{required_space}` - 所需空间

**语言键：**
- `{welcome_message}` - 来自 locales/*.json
- `{install_button}` - 来自 locales/*.json

**示例：**
```xml
<Label text="欢迎使用 {product_name}" />
<Label text="版本：{version}" />
<Button text="{install_button}" />
```

## 验证规则

### 必需元素

- `<Layout>` 必须是根元素
- `<Layout>` 必须包含一个 `<Page>` 元素
- `<Layout>` 必须有 `name` 和 `version` 属性

### 必需属性

- `<Button>` 必须有 `id` 和 `text`
- `<Label>` 必须有 `text`
- `<Image>` 必须有 `icon`
- `<Checkbox>` 必须有 `id` 和 `text`
- `<TextInput>` 必须有 `id`
- `<ProgressBar>` 必须有 `id`

### ID 唯一性

- 同一布局文件中，所有 `id` 属性必须唯一
- `id` 推荐使用 snake_case 命名

### 属性约束

- `width` 和 `height` 必须 > 0 或为 "auto"
- `progress` 必须在 0.0 ~ 1.0 之间
- `font_size` 必须 > 0
- 颜色值必须是有效的十六进制或命名颜色
- 路径必须指向存在的文件

### 嵌套规则

- `<Spacer>`, `<Button>`, `<Label>`, `<Image>`, `<Checkbox>`, `<TextInput>`, `<ProgressBar>`, `<Divider>` 不能包含子元素
- `<VBox>` 和 `<HBox>` 可以包含任意元素
- `<Page>` 可以包含任意元素

## 完整示例

### 基本页面

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Layout name="Welcome" version="1.0.0">
  <Page id="welcome" background="#1A1D28">
    <VBox padding="40,40,40,40" spacing="20" align="center">
      <!-- Logo -->
      <Image icon="assets/logo.png" width="200" height="60" />
      
      <Spacer height="30" />
      
      <!-- 标题 -->
      <Label 
        text="欢迎使用 {product_name}" 
        font_size="28" 
        color="#FFFFFF"
        bold="true"
        align="center"
      />
      
      <!-- 版本 -->
      <Label 
        text="版本 {version}" 
        font_size="14" 
        color="#AAAAAA"
        align="center"
      />
      
      <Spacer height="20" />
      
      <!-- 描述 -->
      <Label 
        text="{welcome_description}" 
        font_size="14" 
        color="#CCCCCC"
        align="center"
        width="400"
        wrap="true"
      />
      
      <Spacer height="40" />
      
      <!-- 按钮 -->
      <HBox spacing="15">
        <Button id="next" text="{next_button}" width="140" height="44" style="primary" />
        <Button id="cancel" text="{cancel_button}" width="140" height="44" />
      </HBox>
    </VBox>
  </Page>
</Layout>
```

### 复杂布局

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Layout name="InstallPath" version="1.0.0">
  <Page id="install_path">
    <VBox padding="40,40,40,40" spacing="15">
      <!-- 标题 -->
      <Label text="{install_location_title}" font_size="20" bold="true" />
      
      <Spacer height="10" />
      
      <!-- 路径输入 -->
      <HBox spacing="10">
        <TextInput id="install_path" text="{install_path}" width="400" readonly="true" />
        <Button id="browse" text="{browse_button}" width="80" height="32" />
      </HBox>
      
      <!-- 空间信息 -->
      <Label 
        text="{required_space_label}: {required_space} MB" 
        font_size="12" 
        color="#888888"
      />
      
      <Spacer height="20" />
      <Divider />
      <Spacer height="20" />
      
      <!-- 选项 -->
      <VBox spacing="12">
        <Label text="{installation_options}" font_size="16" bold="true" />
        <Checkbox id="desktop_shortcut" text="{create_desktop_shortcut}" checked="true" />
        <Checkbox id="start_menu" text="{create_start_menu}" checked="true" />
        <Checkbox id="auto_start" text="{auto_start_option}" checked="false" />
      </VBox>
      
      <Spacer height="auto" />
      
      <!-- 底部按钮 -->
      <HBox spacing="15">
        <Button id="back" text="{back_button}" width="120" />
        <Spacer width="auto" />
        <Button id="install" text="{install_button}" width="140" style="primary" />
        <Button id="cancel" text="{cancel_button}" width="120" />
      </HBox>
    </VBox>
  </Page>
</Layout>
```

## 相关文档

- [XML 布局指南](XML_LAYOUT_GUIDE.md) - 实用教程和示例
- [配置参考](CONFIG_REFERENCE.md) - 配置文件说明
- [多语言支持](LOCALIZATION.md) - 语言字符串引用

---

有问题？查看 [主文档](../README.md) 或提交 Issue。

