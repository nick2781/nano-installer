# XML 布局参考手册

> 本文档是 nano-installer XML 布局系统的完整参考手册，面向安装器界面开发者。

**重要提示**：nano-installer 使用 **NSIS 布局格式**，可以直接使用 NSIS 的 XML 布局文件。

## 📑 目录

- [快速开始](#-快速开始)
- [NSIS 格式支持](#nsis-格式支持)
- [基本概念](#-基本概念)
- [容器元素](#-容器元素)
- [可视化组件](#-可视化组件)
- [通用属性](#-通用属性)
- [图片资源配置](#-图片资源配置)
- [完整示例](#-完整示例)
- [常见问题](#-常见问题)

---

## 🚀 快速开始

### 第一个布局文件

创建 `layouts/welcome.xml`：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Windows>
  <Font id="0" name="微软雅黑" size="14" bold="false" default="true" />
  <VerticalLayout>
    <Label name="title" text="欢迎安装" font="0" textcolor="#FFFFFF" />
    <Label name="message" text="点击下一步继续" font="0" textcolor="#CCCCCC" />
    
    <Container height="1" />
    
    <HorizontalLayout>
      <Container width="1" />
      <Button name="next" text="下一步" width="100" height="40" font="0" />
    </HorizontalLayout>
  </VerticalLayout>
</Windows>
```

## 布局格式

nano-installer 使用 **NSIS 布局格式**，这意味着：

✅ **可以直接使用 NSIS 的布局文件**，无需任何转换  
✅ **与 NSIS 完全一致**，便于参考和调试

### 格式说明

| 特性 | NSIS 格式 |
|------|---------|-----------|
| 根元素 | `<Layout><Page>` | `<Windows>` 或 `<Window>` |
| 垂直布局 | `<VBox>` | `<VerticalLayout>` |
| 水平布局 | `<HBox>` | `<HorizontalLayout>` |
| 空白占位 | `<Spacer>` | `<Container>` 或 `<Control>` |
| 复选框 | `<Checkbox>` | `<CheckBox>` |
| 文本输入 | `<TextInput>` | `<RichEdit>` |
| 进度条 | `<ProgressBar>` | `<Slider>` |
| 元素 ID | `id` | `name` |
| 背景颜色 | `background` | `bkcolor` |
| 背景图片 | `background` | `bkimage` |
| 文本颜色 | `color` | `textcolor` |
| 内边距 | `padding="top,right,bottom,left"` | `padding="left,top,right,bottom"` 或 `inset="left,top,right,bottom"` |
| 字体 | `font_size="14"` | `font="0"`（引用 `<Font>` 元素） |

### 字体系统

NSIS 使用 `<Font>` 元素定义字体：

```xml
<Windows>
  <Font id="0" name="微软雅黑" size="14" bold="false" default="true" />
  <Font id="1" name="微软雅黑" size="20" bold="true" />
  
  <VerticalLayout>
    <Label text="标题" font="1" />
    <Label text="正文" font="0" />
  </VerticalLayout>
</Windows>
```

### 图片路径格式

NSIS 支持复杂的图片路径格式：

```xml
<!-- 基本格式 -->
<Button normalimage="file='assets/btn_primary@2x.png'" />

<!-- 带裁剪区域 -->
<Button normalimage="file='assets/checkbox-0@2x.png' dest='0,2,32,34'" />

<!-- 带圆角和透明度 -->
<Button normalimage="file='assets/btn_primary.png' corner='18,6,18,14' fade='230'" />
```

**格式说明**：
- `file='path'` - 图片路径（必需）
- `dest='x1,y1,x2,y2'` - 裁剪区域（可选）
- `corner='x1,y1,x2,y2'` - 圆角参数（可选）
- `fade='value'` - 透明度 0-255（可选）

### 在配置中引用

编辑 `installer_config.json`：

```json
{
  "ui": {
    "pages": [
      {
        "id": "welcome",
        "layout": "layouts/welcome.xml"
      }
    ]
  }
}
```

---

## 📖 基本概念

### 布局结构

**传统格式：**
```
Layout (布局根节点)
  └── Page (页面容器)
       └── VBox/HBox (容器)
            ├── Label (标签)
            ├── Button (按钮)
            ├── Checkbox (复选框)
            └── ... (其他组件)
```

**NSIS 格式：**
```
Windows (布局根节点)
  ├── Font (字体定义，可选)
  └── VerticalLayout/HorizontalLayout (容器)
       ├── Label (标签)
       ├── Button (按钮)
       ├── CheckBox (复选框)
       └── ... (其他组件)
```

### 坐标系统

- **原点**：左上角 (0, 0)
- **X 轴**：向右递增
- **Y 轴**：向下递增
- **单位**：像素 (px)

### 尺寸单位

所有尺寸单位均为**像素 (px)**，无需后缀：

```xml
<Button width="100" height="40" />  <!-- 100px × 40px -->
```

---

## 📦 容器元素

容器用于组织和排列子元素。

### `<Layout>` / `<Windows>` / `<Window>` - 布局根节点

**必须**作为 XML 文件的根元素。

#### 传统格式：`<Layout>`

| 属性 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | String | ✅ | 布局名称，用于识别 |
| `version` | String | ✅ | 布局版本，如 "1.0.0" |

**示例：**
```xml
<Layout name="Welcome" version="1.0.0">
  <Page>
    <!-- 页面内容 -->
  </Page>
</Layout>
```

#### NSIS 格式：`<Windows>` 或 `<Window>`

**`<Windows>`** - 页面布局根元素（无属性）

**示例：**
```xml
<Windows>
  <Font id="0" name="微软雅黑" size="14" />
  <VerticalLayout>
    <!-- 页面内容 -->
  </VerticalLayout>
</Windows>
```

**`<Window>`** - 窗口定义（用于对话框）

| 属性 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | String | ❌ | 窗口名称 |
| `size` | String | ❌ | 窗口大小，格式 `"width,height"` |
| `roundcorner` | String | ❌ | 圆角，格式 `"x,y"` |
| `showshadow` | Boolean | ❌ | 是否显示阴影 |

**示例：**
```xml
<Window name="msgbox" size="800,460" roundcorner="32,32" showshadow="true">
  <Font id="0" name="微软雅黑" size="14" />
  <VerticalLayout>
    <!-- 对话框内容 -->
  </VerticalLayout>
</Window>
```

---

### `<Page>` - 页面容器

页面的顶层容器，通常只包含一个子元素（通常是 VBox 或 HBox）。

#### 属性

| 属性 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `background` | Color | ❌ | 背景颜色，格式 `#RRGGBB` |
| `background_image` | String | ❌ | 背景图片路径 |

#### 示例

```xml
<Page background="#F5F5F5">
  <VBox>
    <!-- 内容 -->
  </VBox>
</Page>

<Page background_image="assets/background.png">
  <VBox>
    <!-- 内容 -->
  </VBox>
</Page>
```

---

### `<VBox>` / `<VerticalLayout>` - 垂直布局容器

垂直排列子元素（从上到下）。两种格式都支持。

#### 属性

**传统格式 (`<VBox>`)**：

| 属性 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `spacing` | Number | ❌ | 子元素间距，默认 0 |
| `padding` | String | ❌ | 内边距，格式 `"top,right,bottom,left"` |
| `width` | Number | ❌ | 固定宽度 |
| `height` | Number | ❌ | 固定高度 |
| `flex` | Number | ❌ | 弹性增长系数 |
| `align` | String | ❌ | 水平对齐：`"left"` \| `"center"` \| `"right"` |

**NSIS 格式 (`<VerticalLayout>`)**：

| 属性 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | String | ❌ | 元素名称 |
| `width` | Number | ❌ | 固定宽度 |
| `height` | Number | ❌ | 固定高度 |
| `bkcolor` | String | ❌ | 背景颜色，格式 `"#AARRGGBB"` 或 `"#RRGGBB"` |
| `bkimage` | String | ❌ | 背景图片路径 |
| `inset` | String | ❌ | 内边距，格式 `"left,top,right,bottom"` |
| `padding` | String | ❌ | 内边距，格式 `"left,top,right,bottom"` |
| `bordercolor` | String | ❌ | 边框颜色 |
| `bordersize` | Number | ❌ | 边框大小（像素） |
| `borderround` | String | ❌ | 边框圆角，格式 `"x,y"` |
| `visible` | Boolean | ❌ | 是否可见 |
| `float` | Boolean | ❌ | 是否浮动（绝对定位） |
| `pos` | String | ❌ | 浮动位置，格式 `"x1,y1,x2,y2"`（矩形区域） |
| `align` | String | ❌ | 水平对齐：`"left"` \| `"center"` \| `"right"` |
| `valign` | String | ❌ | 垂直对齐：`"top"` \| `"center"` \| `"vcenter"` \| `"bottom"` |

#### 示例

```xml
<!-- 基本用法 -->
<VBox spacing="10" padding="20">
  <Label text="第一行" />
  <Label text="第二行" />
  <Label text="第三行" />
</VBox>

<!-- 独立设置内边距 -->
<VBox padding_left="30" padding_right="30" padding_top="20" padding_bottom="20">
  <Label text="内容" />
</VBox>

<!-- Flex 布局 -->
<HBox>
  <VBox flex="1" padding="10">
    <Label text="左侧列" />
  </VBox>
  <VBox flex="2" padding="10">
    <Label text="右侧列（占 2 倍空间）" />
  </VBox>
</HBox>
```

---

### `<HBox>` / `<HorizontalLayout>` - 水平布局容器

水平排列子元素（从左到右）。两种格式都支持。

#### 属性

与 `<VBox>` / `<VerticalLayout>` 相同。

#### 示例

```xml
<!-- 按钮组 -->
<HBox spacing="10">
  <Button text="取消" text_i18n="button.cancel" />
  <Button text="确定" text_i18n="button.ok" />
</HBox>

<!-- 右对齐按钮 -->
<HBox spacing="10">
  <Spacer flex="1" />
  <Button text="上一步" text_i18n="button.back" />
  <Button text="下一步" text_i18n="button.next" />
</HBox>

<!-- 左中右三栏 -->
<HBox>
  <VBox flex="1">
    <Label text="左侧" />
  </VBox>
  <Spacer width="20" />
  <VBox flex="2">
    <Label text="中间（宽度是左侧的 2 倍）" />
  </VBox>
  <Spacer width="20" />
  <VBox flex="1">
    <Label text="右侧" />
  </VBox>
</HBox>
```

---

### `<Spacer>` / `<Container>` / `<Control>` - 空白占位符

用于创建空白间隔或实现 Flex 布局。三种格式都支持。

#### 属性

**传统格式 (`<Spacer>`)**：

| 属性 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `width` | Number | ❌ | 固定宽度 |
| `height` | Number | ❌ | 固定高度 |
| `flex` | Number | ❌ | 弹性增长系数 |

**NSIS 格式 (`<Container>` 和 `<Control>`)**：

| 属性 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | String | ❌ | 元素名称 |
| `width` | Number | ❌ | 固定宽度 |
| `height` | Number | ❌ | 固定高度 |
| `bkcolor` | String | ❌ | 背景颜色（仅 `<Container>` 和 `<Control>`） |
| `bkimage` | String | ❌ | 背景图片（仅 `<Container>` 和 `<Control>`） |
| `padding` | String | ❌ | 内边距（仅 `<Control>`） |
| `visible` | Boolean | ❌ | 是否可见 |

#### 示例

```xml
<!-- 固定间距 -->
<VBox>
  <Label text="标题" />
  <Spacer height="20" />
  <Label text="内容" />
</VBox>

<!-- 弹性间距（占据剩余空间） -->
<VBox>
  <Label text="顶部内容" />
  <Spacer flex="1" />
  <Label text="底部内容" />
</VBox>

<!-- 按钮右对齐 -->
<HBox>
  <Spacer flex="1" />
  <Button text="确定" />
</HBox>

<!-- 标题居中 -->
<HBox>
  <Spacer flex="1" />
  <Label text="居中标题" />
  <Spacer flex="1" />
</HBox>
```

---

## 🎨 可视化组件

### `<Label>` - 文本标签

显示文本内容。

#### 属性

| 属性 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `text` | String | ✅* | 显示的文本内容 |
| `text_i18n` | String | ❌ | 国际化翻译键（推荐） |
| `font_size` | Number | ❌ | 字体大小，默认 14 |
| `color` | Color | ❌ | 文字颜色，格式 `#RRGGBB` |
| `wrap` | Boolean | ❌ | 是否自动换行，默认 false |
| `max_lines` | Number | ❌ | 最大行数（需配合 wrap） |
| `width` | Number | ❌ | 固定宽度 |
| `min_width` | Number | ❌ | 最小宽度 |
| `max_width` | Number | ❌ | 最大宽度 |
| `height` | Number | ❌ | 固定高度 |
| `flex` | Number | ❌ | 弹性增长系数 |

\* 如果设置了 `text_i18n`，`text` 可选（作为兜底文案）

#### 示例

```xml
<!-- 基本文本 -->
<Label text="安装向导" />

<!-- 国际化文本 -->
<Label text="Install Wizard" text_i18n="wizard.title" />

<!-- 自定义样式 -->
<Label text="重要提示" 
       text_i18n="notice.important"
       font_size="20" 
       color="#FF0000" />

<!-- 长文本自动换行 -->
<Label text="这是一段很长的文字，会根据容器宽度自动换行..." 
       text_i18n="agreement.text"
       wrap="true" 
       width="400" />

<!-- 限制最大行数 -->
<Label text="很长的描述..." 
       text_i18n="app.description"
       wrap="true"
       max_lines="3"
       width="400" />

<!-- 次要文本（灰色、小字号） -->
<Label text="版本 1.0.0" 
       text_i18n="app.version"
       font_size="12" 
       color="#999999" />
```

---

### `<Button>` - 按钮

可点击的按钮组件。

#### 属性

| 属性 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | String | ✅ | 按钮 ID，用于事件处理 |
| `text` | String | ✅* | 按钮文本 |
| `text_i18n` | String | ❌ | 国际化翻译键（推荐） |
| `width` | Number | ❌ | 固定宽度 |
| `min_width` | Number | ❌ | 最小宽度（推荐） |
| `height` | Number | ❌ | 高度，默认 40 |
| `image` | String | ❌ | 按钮图片（默认状态） |
| `image_hover` | String | ❌ | 鼠标悬停时的图片 |
| `image_pressed` | String | ❌ | 按下时的图片 |
| `enabled` | Boolean | ❌ | 是否可用，默认 true |

\* 如果设置了 `text_i18n`，`text` 可选

#### 示例

```xml
<!-- 基本按钮 -->
<Button id="btn_next" text="下一步" />

<!-- 国际化按钮（推荐） -->
<Button id="btn_next" 
        text="Next" 
        text_i18n="button.next" />

<!-- 固定宽度 -->
<Button id="btn_install" 
        text="Install" 
        text_i18n="button.install"
        width="120" 
        height="40" />

<!-- 最小宽度（多语言推荐） -->
<Button id="btn_install" 
        text="Install" 
        text_i18n="button.install"
        min_width="100" 
        height="40" />

<!-- 使用图片按钮 -->
<Button id="btn_install"
        text="Install"
        text_i18n="button.install"
        image="assets/btn_primary.png"
        image_hover="assets/btn_hover.png"
        image_pressed="assets/btn_pressed.png"
        width="200"
        height="50" />

<!-- 禁用按钮 -->
<Button id="btn_next" 
        text="Next" 
        text_i18n="button.next"
        enabled="false" />

<!-- 按钮组 -->
<HBox spacing="10">
  <Spacer flex="1" />
  <Button id="btn_cancel" text="取消" text_i18n="button.cancel" min_width="80" />
  <Button id="btn_back" text="上一步" text_i18n="button.back" min_width="80" />
  <Button id="btn_next" text="下一步" text_i18n="button.next" min_width="80" />
</HBox>
```

#### 按钮图片说明

按钮支持三种状态的图片：

| 状态 | 属性 | 说明 |
|------|------|------|
| 默认 | `image` | 按钮的默认外观 |
| 悬停 | `image_hover` | 鼠标悬停时的外观 |
| 按下 | `image_pressed` | 鼠标按下时的外观 |

**建议尺寸**：
- 标准按钮：200×50 px
- 小按钮：120×40 px
- 图标按钮：32×32 px 或 48×48 px

**设计要点**：
- 默认状态应清晰可识别
- 悬停状态应有明显的视觉反馈（如高亮）
- 按下状态应有按下效果（如阴影变化）

---

### `<Checkbox>` - 复选框

可勾选/取消勾选的选项。

#### 属性

| 属性 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | String | ✅ | 复选框 ID |
| `text` | String | ✅* | 复选框标签文本 |
| `text_i18n` | String | ❌ | 国际化翻译键（推荐） |
| `checked` | Boolean | ❌ | 是否默认选中，默认 false |
| `enabled` | Boolean | ❌ | 是否可用，默认 true |
| `image_unchecked` | String | ❌ | 未选中状态图片 |
| `image_checked` | String | ❌ | 选中状态图片 |

\* 如果设置了 `text_i18n`，`text` 可选

#### 示例

```xml
<!-- 基本复选框 -->
<Checkbox id="agree" text="我同意用户协议" />

<!-- 国际化复选框 -->
<Checkbox id="agree" 
          text="I agree to the terms" 
          text_i18n="agreement.accept" />

<!-- 默认选中 -->
<Checkbox id="desktop_shortcut" 
          text="Create desktop shortcut" 
          text_i18n="option.desktop_shortcut"
          checked="true" />

<!-- 禁用状态 -->
<Checkbox id="option_disabled" 
          text="Disabled option" 
          text_i18n="option.disabled"
          enabled="false" />

<!-- 自定义图片 -->
<Checkbox id="custom" 
          text="Custom checkbox" 
          text_i18n="option.custom"
          image_unchecked="assets/checkbox-0.png"
          image_checked="assets/checkbox-2.png" />

<!-- 复选框列表 -->
<VBox spacing="10">
  <Checkbox id="desktop" text="创建桌面快捷方式" text_i18n="option.desktop" checked="true" />
  <Checkbox id="start_menu" text="添加到开始菜单" text_i18n="option.start_menu" checked="true" />
  <Checkbox id="autostart" text="开机自动启动" text_i18n="option.autostart" checked="false" />
  <Checkbox id="file_association" text="关联文件类型" text_i18n="option.file_association" checked="false" />
</VBox>
```

#### 复选框图片说明

复选框支持两种状态的图片：

| 状态 | 属性 | 说明 |
|------|------|------|
| 未选中 | `image_unchecked` | 复选框未勾选时的外观 |
| 选中 | `image_checked` | 复选框勾选后的外观 |

**建议尺寸**：
- 标准复选框：20×20 px 或 24×24 px
- 大号复选框：32×32 px

**设计要点**：
- 未选中状态通常是空框或灰色框
- 选中状态应有明显的勾选标记（如 ✓）
- 建议提供 @2x 高清版本

---

### `<TextInput>` - 文本输入框

单行文本输入组件。

#### 属性

| 属性 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | String | ✅ | 输入框 ID |
| `text` | String | ❌ | 默认文本内容 |
| `placeholder` | String | ❌ | 占位符文本 |
| `placeholder_i18n` | String | ❌ | 占位符国际化键 |
| `width` | Number | ❌ | 固定宽度 |
| `min_width` | Number | ❌ | 最小宽度 |
| `height` | Number | ❌ | 高度，默认 32 |
| `flex` | Number | ❌ | 弹性增长系数 |
| `enabled` | Boolean | ❌ | 是否可用，默认 true |
| `readonly` | Boolean | ❌ | 是否只读，默认 false |

#### 示例

```xml
<!-- 基本输入框 -->
<TextInput id="username" placeholder="请输入用户名" />

<!-- 国际化占位符 -->
<TextInput id="username" 
           placeholder="Enter username" 
           placeholder_i18n="input.username" />

<!-- 带默认值 -->
<TextInput id="install_path" 
           text="C:\Program Files\MyApp" 
           width="400" />

<!-- 只读输入框 -->
<TextInput id="product_key" 
           text="XXXXX-XXXXX-XXXXX-XXXXX" 
           readonly="true"
           width="300" />

<!-- 弹性宽度输入框 -->
<HBox spacing="10">
  <Label text="路径：" />
  <TextInput id="path" flex="1" />
  <Button id="btn_browse" text="浏览..." width="80" />
</HBox>

<!-- 完整的路径选择示例 -->
<VBox spacing="5">
  <Label text="安装路径：" text_i18n="config.install_path" />
  <HBox spacing="10">
    <TextInput id="install_path" 
               text="{default_path}" 
               flex="1" 
               height="36" />
    <Button id="btn_browse" 
            text="浏览..." 
            text_i18n="button.browse"
            width="80" 
            height="36" />
  </HBox>
  <Label text="需要至少 500 MB 可用空间" 
         text_i18n="config.space_required"
         font_size="12" 
         color="#999999" />
</VBox>
```

---

### `<Image>` - 图片

显示图片资源。

#### 属性

| 属性 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `src` | String | ✅ | 图片路径 |
| `width` | Number | ❌ | 显示宽度 |
| `height` | Number | ❌ | 显示高度 |

#### 示例

```xml
<!-- 基本图片 -->
<Image src="assets/logo.png" />

<!-- 指定尺寸 -->
<Image src="assets/logo.png" width="200" height="200" />

<!-- Logo + 标题组合 -->
<VBox spacing="10">
  <HBox>
    <Spacer flex="1" />
    <Image src="assets/logo.png" width="100" height="100" />
    <Spacer flex="1" />
  </HBox>
  <HBox>
    <Spacer flex="1" />
    <Label text="TapTap 安装向导" text_i18n="app.title" font_size="24" />
    <Spacer flex="1" />
  </HBox>
</VBox>

<!-- 侧边栏图片 -->
<HBox>
  <Image src="assets/sidebar.png" width="200" />
  <VBox flex="1" padding="30">
    <Label text="欢迎" />
  </VBox>
</HBox>
```

---

### `<ProgressBar>` - 进度条

显示进度条。

#### 属性

| 属性 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | String | ✅ | 进度条 ID |
| `value` | Number | ❌ | 当前进度（0-100），默认 0 |
| `width` | Number | ❌ | 宽度 |
| `height` | Number | ❌ | 高度，默认 24 |
| `flex` | Number | ❌ | 弹性增长系数 |
| `image_background` | String | ❌ | 进度条背景图片 |
| `image_bar` | String | ❌ | 进度条填充图片 |

#### 示例

```xml
<!-- 基本进度条 -->
<ProgressBar id="install_progress" width="400" />

<!-- 带初始值 -->
<ProgressBar id="install_progress" value="30" width="400" height="30" />

<!-- 弹性宽度 -->
<ProgressBar id="install_progress" flex="1" height="24" />

<!-- 自定义图片 -->
<ProgressBar id="install_progress" 
             width="500" 
             height="32"
             image_background="assets/progress_bg.png"
             image_bar="assets/progress_bar.png" />

<!-- 完整的安装进度示例 -->
<VBox spacing="15" padding="30">
  <Label text="正在安装..." text_i18n="installing.title" font_size="20" />
  
  <ProgressBar id="install_progress" flex="1" height="32" />
  
  <Label id="install_status" 
         text="正在解压文件..." 
         text_i18n="installing.status"
         font_size="14" 
         color="#666666" />
  
  <Label id="install_detail" 
         text="文件: setup.exe (1.2 MB / 5.0 MB)" 
         font_size="12" 
         color="#999999" />
</VBox>
```

---

## 🔧 通用属性

所有可视化组件都支持以下通用属性：

### 尺寸属性

| 属性 | 类型 | 说明 |
|------|------|------|
| `width` | Number | 固定宽度 |
| `height` | Number | 固定高度 |
| `min_width` | Number | 最小宽度（推荐用于按钮） |
| `max_width` | Number | 最大宽度（推荐用于文本） |
| `min_height` | Number | 最小高度 |
| `max_height` | Number | 最大高度 |

### 布局属性

| 属性 | 类型 | 说明 |
|------|------|------|
| `flex` | Number | 弹性增长系数（0-100） |
| `padding` | Number | 内边距（四周相同） |
| `padding_left` | Number | 左内边距 |
| `padding_right` | Number | 右内边距 |
| `padding_top` | Number | 上内边距 |
| `padding_bottom` | Number | 下内边距 |

### 文本属性

| 属性 | 类型 | 说明 |
|------|------|------|
| `text` | String | 文本内容 |
| `text_i18n` | String | 国际化翻译键（推荐） |
| `font_size` | Number | 字体大小 |
| `color` | Color | 文字颜色 `#RRGGBB` |
| `wrap` | Boolean | 是否自动换行 |
| `max_lines` | Number | 最大行数 |

### 国际化属性

| 属性 | 类型 | 说明 |
|------|------|------|
| `text_i18n` | String | 文本国际化键 |
| `placeholder_i18n` | String | 占位符国际化键 |

**国际化查找顺序**：
1. 查找 `text_i18n` 对应的翻译
2. 如果未找到，使用 `text` 兜底
3. 如果 `text` 也没有，显示空字符串

**推荐做法**：同时设置 `text` 和 `text_i18n`
```xml
<Label text="Welcome" text_i18n="welcome.title" />
<!-- 如果翻译文件中没有 welcome.title，显示 "Welcome" -->
```

---

## 🖼️ 图片资源配置

### 资源目录结构

```
your-project/
├── assets/
│   ├── logo.png                  # 应用图标
│   ├── logo@2x.png               # 高清版本（可选）
│   ├── background.png            # 背景图片
│   ├── background@2x.png
│   │
│   ├── buttons/                  # 按钮图片
│   │   ├── btn_primary.png       # 主按钮（默认状态）
│   │   ├── btn_primary@2x.png
│   │   ├── btn_hover.png         # 悬停状态
│   │   ├── btn_hover@2x.png
│   │   ├── btn_pressed.png       # 按下状态
│   │   ├── btn_pressed@2x.png
│   │   ├── btn_disabled.png      # 禁用状态
│   │   └── btn_disabled@2x.png
│   │
│   ├── checkboxes/               # 复选框图片
│   │   ├── checkbox_unchecked.png
│   │   ├── checkbox_unchecked@2x.png
│   │   ├── checkbox_checked.png
│   │   └── checkbox_checked@2x.png
│   │
│   └── progress/                 # 进度条图片
│       ├── progress_bg.png       # 进度条背景
│       ├── progress_bg@2x.png
│       ├── progress_bar.png      # 进度条填充
│       └── progress_bar@2x.png
```

### 图片命名规范

#### 1. 高清屏支持

使用 `@2x` 后缀标识高清版本：

```
logo.png        → 标准分辨率（100×100 px）
logo@2x.png     → 高清分辨率（200×200 px）
```

**系统会自动选择合适的版本**。

#### 2. 状态命名

**按钮状态**：
- 默认：`btn_xxx.png` 或 `btn_xxx_normal.png`
- 悬停：`btn_xxx_hover.png`
- 按下：`btn_xxx_pressed.png` 或 `btn_xxx_active.png`
- 禁用：`btn_xxx_disabled.png`

**复选框状态**：
- 未选中：`checkbox_unchecked.png` 或 `checkbox_0.png`
- 选中：`checkbox_checked.png` 或 `checkbox_2.png`
- 半选中（可选）：`checkbox_indeterminate.png` 或 `checkbox_1.png`

### 图片格式和尺寸建议

#### 支持的格式

| 格式 | 支持 | 说明 |
|------|------|------|
| PNG | ✅ | 推荐，支持透明 |
| JPG | ✅ | 适合大背景图 |
| BMP | ✅ | 不推荐 |
| GIF | ❌ | 不支持动画 |

#### 推荐尺寸

| 组件类型 | 标准尺寸 | 高清尺寸 (@2x) |
|----------|----------|----------------|
| 应用图标 | 64×64 | 128×128 |
| 大图标 | 128×128 | 256×256 |
| 标准按钮 | 200×50 | 400×100 |
| 小按钮 | 120×40 | 240×80 |
| 图标按钮 | 32×32 | 64×64 |
| 复选框 | 20×20 | 40×40 |
| 复选框（大号） | 32×32 | 64×64 |
| 背景图片 | 800×600 | 1600×1200 |

### 按钮图片示例

#### 完整的按钮状态

```xml
<Button id="btn_install"
        text="立即安装"
        text_i18n="button.install"
        width="200"
        height="50"
        image="assets/buttons/btn_primary.png"
        image_hover="assets/buttons/btn_hover.png"
        image_pressed="assets/buttons/btn_pressed.png" />
```

#### 图片设计要点

**默认状态** (`btn_primary.png`)：
- 清晰可识别的按钮外观
- 合适的对比度
- 建议使用品牌主色

**悬停状态** (`btn_hover.png`)：
- 比默认状态更亮或更深
- 明显的视觉反馈
- 建议添加发光效果

**按下状态** (`btn_pressed.png`)：
- 视觉上的"按下"感觉
- 可以添加阴影或降低亮度
- 按钮看起来向下移动

### 复选框图片示例

#### 标准复选框

```xml
<Checkbox id="agree"
          text="我同意服务条款"
          text_i18n="agreement.accept"
          image_unchecked="assets/checkboxes/checkbox_unchecked.png"
          image_checked="assets/checkboxes/checkbox_checked.png" />
```

#### 图片设计要点

**未选中** (`checkbox_unchecked.png`)：
- 空白的方框或圆圈
- 灰色边框
- 透明背景

**选中** (`checkbox_checked.png`)：
- 带有勾选标记（✓）
- 品牌主色填充或边框
- 透明背景

### 进度条图片示例

#### 自定义进度条

```xml
<ProgressBar id="progress"
             width="500"
             height="32"
             image_background="assets/progress/progress_bg.png"
             image_bar="assets/progress/progress_bar.png" />
```

#### 图片设计要点

**背景图** (`progress_bg.png`)：
- 进度条的轨道
- 通常是灰色或浅色
- 圆角矩形

**填充图** (`progress_bar.png`)：
- 进度的填充部分
- 使用品牌主色或渐变
- 会被水平拉伸，设计时注意边缘

**建议**：
- 背景图宽度 = 进度条宽度
- 填充图可以是小块（会平铺）或完整宽度

---

## 📝 完整示例

### 示例 1：欢迎页面

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Layout name="Welcome" version="1.0.0">
  <Page background_image="assets/background.png">
    <VBox spacing="20" padding="40">
      <!-- Logo 和标题 -->
      <HBox>
        <Spacer flex="1" />
        <VBox spacing="15">
          <Image src="assets/logo.png" width="120" height="120" />
          <Label text="TapTap 安装向导" 
                 text_i18n="welcome.title" 
                 font_size="28" 
                 color="#333333" />
        </VBox>
        <Spacer flex="1" />
      </HBox>
      
      <!-- 描述文本 -->
      <VBox spacing="10" padding_left="50" padding_right="50">
        <Label text="欢迎使用 TapTap，发现和下载精品游戏" 
               text_i18n="welcome.subtitle"
               font_size="16"
               color="#666666" />
        
        <Spacer height="10" />
        
        <Label text="安装程序将引导您完成 TapTap 的安装过程。点击\"下一步\"继续，或点击\"取消\"退出安装程序。" 
               text_i18n="welcome.description"
               wrap="true"
               max_width="600"
               color="#888888" />
      </VBox>
      
      <Spacer flex="1" />
      
      <!-- 用户协议 -->
      <VBox spacing="5" padding_left="50" padding_right="50">
        <Checkbox id="agree_terms" 
                  text="我已阅读并同意《用户协议》和《隐私政策》" 
                  text_i18n="welcome.agree"
                  image_unchecked="assets/checkboxes/checkbox_unchecked.png"
                  image_checked="assets/checkboxes/checkbox_checked.png" />
        
        <Label text="点击\"安装\"即表示您同意上述条款" 
               text_i18n="welcome.agreement_note"
               font_size="12"
               color="#999999" />
      </VBox>
      
      <Spacer height="20" />
      
      <!-- 底部按钮 -->
      <HBox spacing="15" padding_left="50" padding_right="50">
        <Button id="btn_custom" 
                text="自定义安装" 
                text_i18n="button.custom_install"
                image="assets/buttons/btn_secondary.png"
                image_hover="assets/buttons/btn_secondary_hover.png"
                image_pressed="assets/buttons/btn_secondary_pressed.png"
                min_width="120"
                height="50" />
        
        <Spacer flex="1" />
        
        <Button id="btn_cancel" 
                text="取消" 
                text_i18n="button.cancel"
                min_width="100"
                height="50" />
        
        <Button id="btn_next" 
                text="安装" 
                text_i18n="button.install"
                image="assets/buttons/btn_primary.png"
                image_hover="assets/buttons/btn_hover.png"
                image_pressed="assets/buttons/btn_pressed.png"
                min_width="120"
                height="50" />
      </HBox>
    </VBox>
  </Page>
</Layout>
```

---

### 示例 2：配置页面

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Layout name="Config" version="1.0.0">
  <Page background="#F5F5F5">
    <VBox spacing="20" padding="40">
      <!-- 标题 -->
      <Label text="选择安装选项" 
             text_i18n="config.title" 
             font_size="24" 
             color="#333333" />
      
      <Spacer height="10" />
      
      <!-- 安装路径 -->
      <VBox spacing="8">
        <Label text="安装位置" 
               text_i18n="config.install_location" 
               font_size="16" 
               color="#555555" />
        
        <HBox spacing="10">
          <TextInput id="install_path" 
                     text="C:\Program Files\TapTap" 
                     flex="1"
                     height="40" />
          
          <Button id="btn_browse" 
                  text="浏览..." 
                  text_i18n="button.browse"
                  width="100"
                  height="40" />
        </HBox>
        
        <Label text="需要 500 MB 可用空间" 
               text_i18n="config.space_required"
               font_size="12"
               color="#999999" />
      </VBox>
      
      <Spacer height="20" />
      
      <!-- 安装选项 -->
      <VBox spacing="8">
        <Label text="快捷方式" 
               text_i18n="config.shortcuts" 
               font_size="16" 
               color="#555555" />
        
        <VBox spacing="12" padding_left="10">
          <Checkbox id="desktop_shortcut" 
                    text="创建桌面快捷方式" 
                    text_i18n="config.desktop_shortcut"
                    checked="true"
                    image_unchecked="assets/checkboxes/checkbox_unchecked.png"
                    image_checked="assets/checkboxes/checkbox_checked.png" />
          
          <Checkbox id="start_menu" 
                    text="添加到开始菜单" 
                    text_i18n="config.start_menu"
                    checked="true"
                    image_unchecked="assets/checkboxes/checkbox_unchecked.png"
                    image_checked="assets/checkboxes/checkbox_checked.png" />
          
          <Checkbox id="quick_launch" 
                    text="添加快速启动图标" 
                    text_i18n="config.quick_launch"
                    checked="false"
                    image_unchecked="assets/checkboxes/checkbox_unchecked.png"
                    image_checked="assets/checkboxes/checkbox_checked.png" />
        </VBox>
      </VBox>
      
      <Spacer height="10" />
      
      <!-- 其他选项 -->
      <VBox spacing="8">
        <Label text="其他选项" 
               text_i18n="config.other_options" 
               font_size="16" 
               color="#555555" />
        
        <VBox spacing="12" padding_left="10">
          <Checkbox id="autostart" 
                    text="开机自动启动" 
                    text_i18n="config.autostart"
                    checked="false"
                    image_unchecked="assets/checkboxes/checkbox_unchecked.png"
                    image_checked="assets/checkboxes/checkbox_checked.png" />
          
          <Checkbox id="file_association" 
                    text="关联 .taplink 文件" 
                    text_i18n="config.file_association"
                    checked="true"
                    image_unchecked="assets/checkboxes/checkbox_unchecked.png"
                    image_checked="assets/checkboxes/checkbox_checked.png" />
        </VBox>
      </VBox>
      
      <Spacer flex="1" />
      
      <!-- 底部按钮 -->
      <HBox spacing="15">
        <Button id="btn_back" 
                text="上一步" 
                text_i18n="button.back"
                min_width="100"
                height="45" />
        
        <Spacer flex="1" />
        
        <Button id="btn_cancel" 
                text="取消" 
                text_i18n="button.cancel"
                min_width="100"
                height="45" />
        
        <Button id="btn_install" 
                text="开始安装" 
                text_i18n="button.start_install"
                image="assets/buttons/btn_primary.png"
                image_hover="assets/buttons/btn_hover.png"
                image_pressed="assets/buttons/btn_pressed.png"
                min_width="120"
                height="45" />
      </HBox>
    </VBox>
  </Page>
</Layout>
```

---

### 示例 3：安装进度页面

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Layout name="Installing" version="1.0.0">
  <Page background="#FFFFFF">
    <VBox spacing="30" padding="50">
      <!-- 标题 -->
      <HBox>
        <Spacer flex="1" />
        <Label text="正在安装 TapTap" 
               text_i18n="installing.title" 
               font_size="26" 
               color="#333333" />
        <Spacer flex="1" />
      </HBox>
      
      <Spacer height="20" />
      
      <!-- 进度条 -->
      <VBox spacing="15">
        <ProgressBar id="install_progress" 
                     value="0"
                     flex="1"
                     height="40"
                     image_background="assets/progress/progress_bg.png"
                     image_bar="assets/progress/progress_bar.png" />
        
        <HBox>
          <Label id="progress_percentage" 
                 text="0%" 
                 font_size="18" 
                 color="#666666" />
          <Spacer flex="1" />
          <Label id="progress_status" 
                 text="准备中..." 
                 text_i18n="installing.preparing"
                 font_size="18" 
                 color="#666666" />
        </HBox>
      </VBox>
      
      <Spacer height="30" />
      
      <!-- 详细信息 -->
      <VBox spacing="10" padding="20" background="#F9F9F9">
        <Label text="当前操作：" 
               text_i18n="installing.current_operation" 
               font_size="14" 
               color="#888888" />
        
        <Label id="current_file" 
               text="正在解压: setup.exe" 
               font_size="14" 
               color="#555555" 
               wrap="true" 
               max_width="600" />
        
        <Spacer height="10" />
        
        <Label text="已完成：" 
               text_i18n="installing.completed" 
               font_size="14" 
               color="#888888" />
        
        <Label id="files_processed" 
               text="15 / 127 个文件" 
               font_size="14" 
               color="#555555" />
        
        <Spacer height="10" />
        
        <Label text="剩余时间：" 
               text_i18n="installing.remaining_time" 
               font_size="14" 
               color="#888888" />
        
        <Label id="time_remaining" 
               text="约 2 分钟" 
               font_size="14" 
               color="#555555" />
      </VBox>
      
      <Spacer flex="1" />
      
      <!-- 提示信息 -->
      <VBox spacing="5">
        <Label text="💡 小提示：安装过程中请勿关闭此窗口" 
               font_size="12" 
               color="#999999" />
        
        <Label text="安装完成后您可以在桌面找到 TapTap 快捷方式" 
               text_i18n="installing.tip"
               font_size="12" 
               color="#999999"
               wrap="true"
               max_width="600" />
      </VBox>
      
      <Spacer height="20" />
      
      <!-- 取消按钮 -->
      <HBox>
        <Spacer flex="1" />
        <Button id="btn_cancel" 
                text="取消安装" 
                text_i18n="button.cancel_install"
                min_width="120"
                height="40" />
      </HBox>
    </VBox>
  </Page>
</Layout>
```

---

## ❓ 常见问题

### 布局相关

**Q: 如何让按钮右对齐？**

A: 使用 Spacer + flex：
```xml
<HBox>
  <Spacer flex="1" />
  <Button text="确定" />
</HBox>
```

**Q: 如何让标题居中？**

A: 两侧加 Spacer：
```xml
<HBox>
  <Spacer flex="1" />
  <Label text="标题" />
  <Spacer flex="1" />
</HBox>
```

**Q: 如何实现左右分栏？**

A: 使用 flex 权重：
```xml
<HBox>
  <VBox flex="1"><!-- 左侧 --></VBox>
  <Spacer width="20" />
  <VBox flex="2"><!-- 右侧，宽度是左侧 2 倍 --></VBox>
</HBox>
```

### 国际化相关

**Q: text 和 text_i18n 有什么区别？**

A: 
- `text_i18n`：国际化翻译键（推荐使用）
- `text`：兜底文案，当翻译键不存在时显示

**Q: 如何测试多语言布局？**

A: 在安装器中切换语言，观察布局是否正常。建议：
- 使用 `min_width` 而不是固定 `width`
- 长文本使用 `wrap="true"`

### 图片相关

**Q: 如何支持高清屏？**

A: 提供 `@2x` 版本：
```
logo.png       → 100×100 px
logo@2x.png    → 200×200 px
```

**Q: 按钮图片的三种状态是什么？**

A:
- `image`：默认状态
- `image_hover`：鼠标悬停
- `image_pressed`：鼠标按下

**Q: 复选框只有两种状态吗？**

A: 通常是两种（选中/未选中），也可支持第三种（半选中），但需要自行实现。

### 性能相关

**Q: 图片会被缓存吗？**

A: 会。相同路径的图片只加载一次。

**Q: 支持动画吗？**

A: 当前版本不支持 GIF 动画，但按钮的悬停/按下切换是即时的。

---

## 📚 相关文档

- [快速开始指南](guides/QUICKSTART.md)
- [配置文件参考](API.md)
- [国际化指南](LOCALIZATION_UPDATE.md)
- [示例项目](../examples/TapTap/)

---

<div align="center">

**[⬆ 回到顶部](#xml-布局参考手册)**

最后更新：2025-11-05

</div>

