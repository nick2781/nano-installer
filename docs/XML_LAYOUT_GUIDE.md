# XML 布局指南

nano-installer 使用 XML 来定义安装程序的用户界面。这份指南将教你如何创建和自定义界面布局。

**重要提示**：nano-installer 完全兼容 NSIS 布局格式，可以直接使用 NSIS 的 XML 布局文件，同时也支持传统的布局格式（向后兼容）。

## 📖 目录

- [基本结构](#基本结构)
- [NSIS 格式支持](#nsis-格式支持)
- [布局元素](#布局元素)
- [元素属性](#元素属性)
- [变量替换](#变量替换)
- [DPI 支持](#dpi-支持)
- [完整示例](#完整示例)

## 基本结构

nano-installer 支持两种布局格式：

### 传统格式（向后兼容）

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Layout name="PageName" version="1.0.0">
  <Page>
    <!-- 页面内容 -->
  </Page>
</Layout>
```

### NSIS 格式（推荐）

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Windows>
  <Font id="0" name="微软雅黑" size="14" bold="false" />
  <VerticalLayout>
    <!-- 页面内容 -->
  </VerticalLayout>
</Windows>
```

### 根元素属性

**传统格式 (`<Layout>`)**：

| 属性 | 必需 | 说明 | 示例 |
|------|------|------|------|
| `name` | 是 | 布局名称 | `"Welcome"` |
| `version` | 是 | 布局版本 | `"1.0.0"` |

**NSIS 格式 (`<Windows>` 或 `<Window>`)**：

| 属性 | 必需 | 说明 | 示例 |
|------|------|------|------|
| 无 | - | `<Windows>` 无属性 | - |
| `name` | 否 | 窗口名称（仅 `<Window>`） | `"msgbox"` |
| `size` | 否 | 窗口大小（仅 `<Window>`） | `"800,460"` |

## NSIS 格式支持

nano-installer **完全兼容** NSIS 布局格式，这意味着：

✅ **可以直接使用 NSIS 的布局文件**，无需任何转换  
✅ **保持向后兼容**，现有布局文件仍然可用  
✅ **与 NSIS 完全一致**，便于参考和调试  
✅ **无需维护两套布局文件**

### 支持的 NSIS 元素

| NSIS 元素 | 说明 | 映射 |
|-----------|------|------|
| `<Windows>` | 页面布局根元素 | 等同于 `<Layout><Page>` |
| `<Window>` | 窗口定义（用于对话框） | 等同于 `<Layout><Page>` |
| `<VerticalLayout>` | 垂直布局容器 | 等同于 `<VBox>` |
| `<HorizontalLayout>` | 水平布局容器 | 等同于 `<HBox>` |
| `<Container>` | 空白占位符 | 等同于 `<Spacer>` |
| `<Control>` | 控制元素（可显示图片） | 等同于 `<Spacer>` |
| `<Button>` | 按钮 | 直接支持 |
| `<CheckBox>` | 复选框 | 等同于 `<Checkbox>` |
| `<Label>` | 文本标签 | 直接支持 |
| `<RichEdit>` | 富文本编辑框 | 等同于 `<TextInput>` |
| `<Slider>` | 滑块/进度条 | 等同于 `<ProgressBar>` |

### 支持的 NSIS 属性

| NSIS 属性 | 说明 | 映射 |
|-----------|------|------|
| `name` | 元素名称 | 映射到 `id` |
| `bkcolor` | 背景颜色 | 映射到 `background` |
| `bkimage` | 背景图片 | 映射到 `background` |
| `textcolor` | 文本颜色 | 映射到 `color` |
| `inset` | 内边距（left,top,right,bottom） | 转换为 `padding`（top,right,bottom,left） |
| `padding` | 内边距（left,top,right,bottom） | 转换为 `padding`（top,right,bottom,left） |
| `margin` | 外边距（left,top,right,bottom） | 存储到 `custom` |
| `textpadding` | 文本内边距（left,top,right,bottom） | 存储到 `custom` |
| `align` | 水平对齐 | 直接支持 |
| `valign` | 垂直对齐 | 存储到 `custom` |
| `textalign` | 文本对齐（Label 专用） | 存储到 `custom` |
| `font` | 字体 ID（引用 `<Font>` 元素） | 转换为字体配置 |
| `borderround` | 圆角（x,y） | 存储到 `custom` |
| `bordercolor` | 边框颜色 | 存储到 `custom` |
| `bordersize` | 边框大小 | 存储到 `custom` |
| `float` | 是否浮动（绝对定位） | 存储到 `custom` |
| `pos` | 位置矩形（x1,y1,x2,y2） | 转换为 `position` + `width` + `height` |

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

### 字体系统

NSIS 使用 `<Font>` 元素定义字体，然后在其他元素中通过 `font="id"` 引用：

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

**字体属性**：
- `id` - 字体 ID（必需，用于引用）
- `name` - 字体名称（可选，默认"微软雅黑"）
- `size` - 字体大小（必需）
- `bold` - 是否粗体（可选，默认 false）
- `default` - 是否默认字体（可选，默认 false）

## 布局元素

### 容器元素

#### `<VBox>` / `<VerticalLayout>` - 垂直布局

将子元素垂直排列。两种格式都支持：

**传统格式：**
```xml
<VBox>
  <Label text="第一行" />
  <Label text="第二行" />
  <Label text="第三行" />
</VBox>
```

**NSIS 格式：**
```xml
<VerticalLayout>
  <Label text="第一行" />
  <Label text="第二行" />
  <Label text="第三行" />
</VerticalLayout>
```

**属性：**
- `padding` - 内边距，格式：`"top,right,bottom,left"`（传统）或 `"left,top,right,bottom"`（NSIS）
- `inset` - 内边距（NSIS 格式，格式：`"left,top,right,bottom"`）
- `spacing` - 子元素间距（像素）
- `align` - 水平对齐：`"left"` | `"center"` | `"right"`
- `valign` - 垂直对齐（NSIS）：`"top"` | `"center"` | `"vcenter"` | `"bottom"`
- `bkcolor` - 背景颜色（NSIS）
- `bkimage` - 背景图片（NSIS）
- `borderround` - 圆角（NSIS，格式：`"x,y"`）
- `bordercolor` - 边框颜色（NSIS）
- `bordersize` - 边框大小（NSIS）
- `float` - 是否浮动定位（NSIS）
- `pos` - 位置矩形（NSIS，格式：`"x1,y1,x2,y2"`）

#### `<HBox>` / `<HorizontalLayout>` - 水平布局

将子元素水平排列。两种格式都支持：

**传统格式：**
```xml
<HBox>
  <Button text="按钮1" />
  <Button text="按钮2" />
  <Button text="按钮3" />
</HBox>
```

**NSIS 格式：**
```xml
<HorizontalLayout>
  <Button text="按钮1" />
  <Button text="按钮2" />
  <Button text="按钮3" />
</HorizontalLayout>
```

**属性：** 同 `<VBox>` / `<VerticalLayout>`

#### `<Spacer>` / `<Container>` / `<Control>` - 空白占位符

创建空白间距或占位符。

**传统格式：**
```xml
<Spacer height="20" />
<Spacer width="10" />
```

**NSIS 格式：**
```xml
<Container width="20" height="20" />
<Control width="10" height="10" bkimage="assets/bg.png" />
```

**属性：**
- `width` - 宽度（像素）
- `height` - 高度（像素）
- `bkcolor` - 背景颜色（NSIS，仅 `<Container>` 和 `<Control>`）
- `bkimage` - 背景图片（NSIS，仅 `<Container>` 和 `<Control>`）
- `visible` - 是否可见

### 基础元素

#### `<Label>` - 文本标签

显示文本内容。两种格式都支持：

**传统格式：**
```xml
<Label 
  text="欢迎使用 {product_name}" 
  font_size="24" 
  color="#FFFFFF"
/>
```

**NSIS 格式：**
```xml
<Label 
  name="title"
  text="欢迎使用 {product_name}" 
  font="1"
  textcolor="#FFFFFF"
  width="400"
  height="50"
  textalign="center"
  align="center"
  valign="center"
  padding="10,10,10,10"
  showhtml="false"
/>
```

**属性：**

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `name` | string | - | 元素名称（NSIS） |
| `text` | string | 必需 | 显示的文本内容 |
| `font_size` | number | 14 | 字体大小（像素，传统格式） |
| `font` | number | - | 字体 ID（NSIS，引用 `<Font>` 元素） |
| `color` / `textcolor` | string | `"#FFFFFF"` | 文本颜色（`color` 传统，`textcolor` NSIS） |
| `bold` | boolean | false | 是否加粗（传统格式） |
| `italic` | boolean | false | 是否斜体（传统格式） |
| `alignment` / `textalign` | string | `"left"` | 文本对齐（`alignment` 传统，`textalign` NSIS）：`"left"` \| `"center"` \| `"right"` |
| `align` | string | - | 水平对齐（NSIS）：`"left"` \| `"center"` \| `"right"` |
| `valign` | string | - | 垂直对齐（NSIS）：`"top"` \| `"center"` \| `"vcenter"` \| `"bottom"` |
| `width` | number | - | 宽度（像素） |
| `height` | number | - | 高度（像素） |
| `padding` | string | - | 内边距（NSIS，格式：`"left,top,right,bottom"`） |
| `showhtml` | boolean | false | 是否显示 HTML（NSIS） |
| `wrap` | boolean | false | 是否自动换行（传统格式） |
| `max_lines` | number | - | 最大行数（传统格式） |

#### `<Button>` - 按钮

可点击的按钮。两种格式都支持：

**传统格式：**
```xml
<Button 
  id="next" 
  text="下一步" 
  width="120" 
  height="40" 
  style="primary"
/>
```

**NSIS 格式：**
```xml
<Button 
  name="next" 
  text="下一步" 
  width="120" 
  height="40"
  font="0"
  textcolor="#FFFFFF"
  normalimage="file='assets/btn_primary@2x.png'"
  hotimage="file='assets/btn_hover@2x.png'"
  pushedimage="file='assets/btn_pressed@2x.png'"
  disabledimage="file='assets/btn_disabled@2x.png'"
  padding="10,5,10,5"
  textpadding="0,0,32,0"
  align="center"
  valign="center"
  borderround="24,24"
/>
```

**属性：**

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `id` / `name` | string | 必需 | 按钮标识符（`id` 传统，`name` NSIS） |
| `text` | string | 必需 | 按钮文字 |
| `width` | number | 120 | 按钮宽度（像素） |
| `height` | number | 40 | 按钮高度（像素） |
| `enabled` | boolean | true | 是否启用 |
| `style` | string | `"default"` | 样式（传统）：`"primary"` \| `"secondary"` \| `"link"` |
| `font` | number | - | 字体 ID（NSIS，引用 `<Font>` 元素） |
| `textcolor` | string | - | 文本颜色（NSIS） |
| `hottextcolor` | string | - | 悬停时文本颜色（NSIS） |
| `pushedtextcolor` | string | - | 按下时文本颜色（NSIS） |
| `disabledtextcolor` | string | - | 禁用时文本颜色（NSIS） |
| `normalimage` | string | - | 正常状态图片（NSIS，支持 `file='path' dest='x1,y1,x2,y2' corner='x1,y1,x2,y2' fade='value'` 格式） |
| `hotimage` | string | - | 悬停状态图片（NSIS） |
| `pushedimage` | string | - | 按下状态图片（NSIS） |
| `disabledimage` | string | - | 禁用状态图片（NSIS） |
| `focusedimage` | string | - | 获得焦点时图片（NSIS） |
| `padding` | string | - | 内边距（NSIS，格式：`"left,top,right,bottom"`） |
| `margin` | string | - | 外边距（NSIS，格式：`"left,top,right,bottom"`） |
| `inset` | string | - | 内边距（NSIS，格式：`"left,top,right,bottom"`） |
| `textpadding` | string | - | 文本内边距（NSIS，格式：`"left,top,right,bottom"`） |
| `align` | string | - | 水平对齐（NSIS）：`"left"` \| `"center"` \| `"right"` |
| `valign` | string | - | 垂直对齐（NSIS）：`"top"` \| `"center"` \| `"vcenter"` \| `"bottom"` |
| `borderround` | string | - | 圆角（NSIS，格式：`"x,y"`） |
| `bordercolor` | string | - | 边框颜色（NSIS） |
| `bordersize` | number | - | 边框大小（NSIS） |
| `cursor` | string | - | 鼠标样式（NSIS）：`"hand"` \| `"arrow"` 等 |
| `float` | boolean | - | 是否浮动定位（NSIS） |
| `pos` | string | - | 位置矩形（NSIS，格式：`"x1,y1,x2,y2"`） |

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

#### `<Checkbox>` / `<CheckBox>` - 复选框

用户可勾选的选项。两种格式都支持：

**传统格式：**
```xml
<Checkbox 
  id="agree_license" 
  text="我同意许可协议" 
  checked="false"
/>
```

**NSIS 格式：**
```xml
<CheckBox 
  name="agree_license" 
  text="我同意许可协议" 
  selected="false"
  normalimage="file='assets/checkbox-0@2x.png' dest='0,2,32,34'"
  selectedimage="file='assets/checkbox-2@2x.png' dest='0,2,32,34'"
/>
```

**属性：**

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `id` / `name` | string | 必需 | 复选框标识符（`id` 传统，`name` NSIS） |
| `text` | string | 必需 | 显示文本 |
| `checked` / `selected` | boolean | false | 初始状态（`checked` 传统，`selected` NSIS） |
| `enabled` | boolean | true | 是否启用 |
| `font` | number | - | 字体 ID（NSIS，引用 `<Font>` 元素） |
| `textcolor` | string | - | 文本颜色（NSIS） |
| `textpadding` | string | - | 文本内边距（NSIS，格式：`"left,top,right,bottom"`） |
| `normalimage` | string | - | 未选中图片（NSIS，支持 `file='path' dest='x1,y1,x2,y2'` 格式） |
| `normalhotimage` | string | - | 未选中悬停图片（NSIS） |
| `selectedimage` | string | - | 选中图片（NSIS） |
| `selectedhotimage` | string | - | 选中悬停图片（NSIS） |
| `align` | string | - | 水平对齐（NSIS） |
| `valign` | string | - | 垂直对齐（NSIS） |

**常用 ID：**

| ID | 功能 |
|-----|------|
| `desktop_shortcut` | 创建桌面快捷方式 |
| `start_menu` | 添加到开始菜单 |
| `auto_start` | 开机自动启动 |
| `agree_license` | 同意许可协议 |

#### `<TextInput>` / `<RichEdit>` - 文本输入框

用户输入文本的控件。两种格式都支持：

**传统格式：**
```xml
<TextInput 
  id="install_path" 
  text="C:\Program Files\MyApp" 
  width="300"
/>
```

**NSIS 格式：**
```xml
<RichEdit 
  name="install_path" 
  text="C:\Program Files\MyApp" 
  width="300"
  font="0"
  textcolor="#FFFFFF"
  bkcolor="#1A1D28"
  readonly="false"
  multiline="false"
/>
```

**属性：**

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `id` / `name` | string | 必需 | 输入框标识符（`id` 传统，`name` NSIS） |
| `text` | string | `""` | 初始文本 |
| `placeholder` | string | `""` | 占位符文本（仅传统格式） |
| `width` | number | 200 | 宽度（像素） |
| `height` | number | 32 | 高度（像素） |
| `readonly` | boolean | false | 是否只读 |
| `multiline` | boolean | false | 是否多行（NSIS） |
| `font` | number | - | 字体 ID（NSIS） |
| `textcolor` | string | - | 文本颜色（NSIS） |
| `bkcolor` | string | - | 背景颜色（NSIS） |
| `inset` | string | - | 内边距（NSIS，格式：`"left,top,right,bottom"`） |
| `borderround` | string | - | 圆角（NSIS，格式：`"x,y"`） |
| `autohscroll` | boolean | - | 自动水平滚动（NSIS） |
| `wantreturn` | boolean | - | 接受回车（NSIS） |
| `wantctrlreturn` | boolean | - | 接受 Ctrl+回车（NSIS） |

#### `<ProgressBar>` / `<Slider>` - 进度条

显示安装进度。两种格式都支持：

**传统格式：**
```xml
<ProgressBar 
  id="install_progress" 
  progress="0.5" 
  width="400" 
  height="8"
/>
```

**NSIS 格式：**
```xml
<Slider 
  name="install_progress" 
  min="0" 
  max="100" 
  value="50" 
  width="400" 
  height="8"
  bkcolor="#1A1D28"
  foreimage="file='assets/progress_fg.png'"
/>
```

**属性：**

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `id` / `name` | string | 必需 | 进度条标识符（`id` 传统，`name` NSIS） |
| `progress` | number | 0.0 | 进度值（0.0 ~ 1.0，传统格式） |
| `min` | number | 0 | 最小值（NSIS） |
| `max` | number | 100 | 最大值（NSIS） |
| `value` | number | 0 | 当前值（NSIS，计算进度：`(value - min) / (max - min)`） |
| `width` | number | 300 | 宽度（像素） |
| `height` | number | 8 | 高度（像素） |
| `bkcolor` | string | - | 背景颜色（NSIS） |
| `foreimage` | string | - | 前景图片（NSIS，支持 `file='path'` 格式） |
| `thumbsize` | string | - | 滑块大小（NSIS，格式：`"width,height"`） |
| `mouse` | boolean | - | 是否可用鼠标（NSIS） |
| `enabled` | boolean | true | 是否启用 |

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
