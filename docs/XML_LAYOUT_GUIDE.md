# XML 布局指南

nano-installer 使用 XML 来定义安装程序的用户界面。这份指南将教你如何创建和自定义界面布局。

**重要提示**：推荐优先使用声明式 XML DSL 来表达结构、盒模型和控件内容。

## 设计原则

布局系统分为三层：

1. XML 负责声明结构和盒模型语义。
2. Taffy 负责计算布局。
3. 渲染器只负责绘制，不再私自决定控件内部排版。

这意味着一个按钮里的“文字在左还是右、图标距离文字多少、超长时是否换行”，都应该由 XML 明确表达，而不是靠 `dest='68,2,80,15'` 这样的像素坐标暗示。

## 推荐的内容 DSL

对于“按钮内部还有自己的内容布局”这种场景，推荐使用 `Button + Content + Text + Icon` 结构：

```xml
<Button min-width="80" max-width="164">
  <Content
    layout="horizontal"
    horizontal-align="right"
    vertical-align="center"
    item-spacing="4">
    <Text value="@show_more" wrap="true" />
    <Icon src="assets/arrow-down.png" width="12" height="12" />
  </Content>
</Button>
```

这套 DSL 的命名规则是：

- `layout`：子项排列方向，`horizontal` 或 `vertical`
- `horizontal-align`：内容在水平方向的对齐方式，`left` / `center` / `right`
- `vertical-align`：内容在垂直方向的对齐方式，`top` / `center` / `bottom`
- `item-spacing`：子项之间的间距，单位像素
- `wrap`：文本是否自动换行

如果只是普通按钮，仍然可以继续使用 `text="..."` 这种简写；只有在按钮内部需要更复杂的盒模型时，才使用内容 DSL。

## 📖 目录

- [基本结构](#基本结构)
- [布局元素](#布局元素)
- [元素属性](#元素属性)
- [变量替换](#变量替换)
- [DPI 支持](#dpi-支持)
- [完整示例](#完整示例)

## 基本结构

nano-installer 使用 XML 布局 DSL，支持以下基本结构：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Windows>
  <Font id="0" name="微软雅黑" size="14" bold="false" />
  <VerticalLayout>
    <!-- 页面内容 -->
  </VerticalLayout>
</Windows>
```

### 根元素

nano-installer 支持两种根元素：

**`<Windows>`**：用于主安装程序界面

```xml
<Windows>
  <VerticalLayout>
    <!-- 页面内容 -->
  </VerticalLayout>
</Windows>
```

**`<Window>`**：用于对话框（如消息框）

```xml
<Window name="msgbox" size="800,460">
  <VerticalLayout>
    <!-- 对话框内容 -->
  </VerticalLayout>
</Window>
```

### 根元素属性

| 属性 | 必需 | 说明 | 示例 |
|------|------|------|------|
| 无 | - | `<Windows>` 无属性 | - |
| `name` | 否 | 窗口名称（仅 `<Window>`） | `"msgbox"` |
| `size` | 否 | 窗口大小（仅 `<Window>`） | `"800,460"` |

## 支持的元素

| 元素 | 说明 |
|------|------|
| `<Windows>` | 页面布局根元素 |
| `<Window>` | 窗口定义（用于对话框） |
| `<VerticalLayout>` | 垂直布局容器 |
| `<HorizontalLayout>` | 水平布局容器 |
| `<Container>` | 空白占位符 |
| `<Control>` | 控制元素（可显示图片） |
| `<Button>` | 按钮 |
| `<CheckBox>` | 复选框 |
| `<Label>` | 文本标签 |
| `<RichEdit>` | 富文本编辑框 |
| `<TabLayout>` | 标签页容器（暂未完全实现） |
| `<Include>` | 包含其他布局文件（暂未完全实现） |

## 支持的属性

以下属性覆盖当前运行时支持的常用字段。部分字段存在别名或另一种写法，便于在不同布局风格下使用。

| 属性 | 说明 |
|------|------|
| `name` | 元素名称（映射到内部 `id`） |
| `bkcolor` | 背景颜色（映射到内部 `background`） |
| `bkimage` | 背景图片（映射到内部 `background`） |
| `textcolor` | 文本颜色（映射到内部 `color`） |
| `inset` | 内边距（left,top,right,bottom，转换为内部格式 top,right,bottom,left） |
| `padding` | 内边距（left,top,right,bottom，转换为内部格式 top,right,bottom,left） |
| `margin` | 外边距（left,top,right,bottom） |
| `textpadding` | 文本内边距（left,top,right,bottom） |
| `align` | 水平对齐 |
| `valign` | 垂直对齐 |
| `textalign` | 文本对齐（Label 专用） |
| `font` | 字体 ID（引用 `<Font>` 元素） | 转换为字体配置 |
| `borderround` | 圆角（x,y） | 存储到 `custom` |
| `bordercolor` | 边框颜色 | 存储到 `custom` |
| `bordersize` | 边框大小 | 存储到 `custom` |
| `float` | 是否浮动（绝对定位） | 存储到 `custom` |
| `pos` | 位置矩形（x1,y1,x2,y2） | 转换为 `position` + `width` + `height` |

### 图片路径格式

图片资源字段支持复杂的字符串格式：

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

可以使用 `<Font>` 元素定义字体，然后在其他元素中通过 `font="id"` 引用：

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

将子元素垂直排列。

**简洁写法：**
```xml
<VBox>
  <Label text="第一行" />
  <Label text="第二行" />
  <Label text="第三行" />
</VBox>
```

**另一种写法：**
```xml
<VerticalLayout>
  <Label text="第一行" />
  <Label text="第二行" />
  <Label text="第三行" />
</VerticalLayout>
```

**属性：**
- `padding` - 内边距，格式：`"top,right,bottom,left"` 或 `"left,top,right,bottom"`
- `inset` - 内边距，格式：`"left,top,right,bottom"`
- `spacing` - 子元素间距（像素）
- `align` - 水平对齐：`"left"` | `"center"` | `"right"`
- `valign` - 垂直对齐：`"top"` | `"center"` | `"vcenter"` | `"bottom"`
- `bkcolor` - 背景颜色
- `bkimage` - 背景图片
- `borderround` - 圆角，格式：`"x,y"`
- `bordercolor` - 边框颜色
- `bordersize` - 边框大小
- `float` - 是否浮动定位
- `pos` - 位置矩形，格式：`"x1,y1,x2,y2"`

#### `<HBox>` / `<HorizontalLayout>` - 水平布局

将子元素水平排列。

**简洁写法：**
```xml
<HBox>
  <Button text="按钮1" />
  <Button text="按钮2" />
  <Button text="按钮3" />
</HBox>
```

**另一种写法：**
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

**简洁写法：**
```xml
<Spacer height="20" />
<Spacer width="10" />
```

**另一种写法：**
```xml
<Container width="20" height="20" />
<Control width="10" height="10" bkimage="assets/bg.png" />
```

**属性：**
- `width` - 宽度（像素）
- `height` - 高度（像素）
- `bkcolor` - 背景颜色（仅 `<Container>` 和 `<Control>`）
- `bkimage` - 背景图片（仅 `<Container>` 和 `<Control>`）
- `visible` - 是否可见

### 基础元素

#### `<Label>` - 文本标签

显示文本内容。

**简洁写法：**
```xml
<Label 
  text="欢迎使用 {product_name}" 
  font_size="24" 
  color="#FFFFFF"
/>
```

**另一种写法：**
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
| `name` | string | - | 元素名称 |
| `text` | string | 必需 | 显示的文本内容 |
| `font_size` | number | 14 | 字体大小（像素） |
| `font` | number | - | 字体 ID（引用 `<Font>` 元素） |
| `color` / `textcolor` | string | `"#FFFFFF"` | 文本颜色（`color` 与 `textcolor` 都可使用） |
| `bold` | boolean | false | 是否加粗 |
| `italic` | boolean | false | 是否斜体 |
| `alignment` / `textalign` | string | `"left"` | 文本对齐（`alignment` 与 `textalign` 都可使用）：`"left"` \| `"center"` \| `"right"` |
| `align` | string | - | 水平对齐：`"left"` \| `"center"` \| `"right"` |
| `valign` | string | - | 垂直对齐：`"top"` \| `"center"` \| `"vcenter"` \| `"bottom"` |
| `width` | number | - | 宽度（像素） |
| `height` | number | - | 高度（像素） |
| `padding` | string | - | 内边距，格式：`"left,top,right,bottom"` |
| `showhtml` | boolean | false | 是否显示 HTML |
| `wrap` | boolean | false | 是否自动换行 |
| `max_lines` | number | - | 最大行数 |

#### `<Button>` - 按钮

可点击的按钮。

**推荐写法：**
```xml
<Button 
  id="next" 
  text="下一步" 
  width="120" 
  height="40" 
  style="primary"
/>
```

**完整写法：**
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
| `id` / `name` | string | 必需 | 按钮标识符（`id` 与 `name` 都可使用） |
| `text` | string | 必需 | 按钮文字 |
| `width` | number | 120 | 按钮宽度（像素） |
| `height` | number | 40 | 按钮高度（像素） |
| `enabled` | boolean | true | 是否启用 |
| `style` | string | `"default"` | 样式：`"primary"` \| `"secondary"` \| `"link"` |
| `font` | number | - | 字体 ID（引用 `<Font>` 元素） |
| `textcolor` | string | - | 文本颜色 |
| `hottextcolor` | string | - | 悬停时文本颜色 |
| `pushedtextcolor` | string | - | 按下时文本颜色 |
| `disabledtextcolor` | string | - | 禁用时文本颜色 |
| `normalimage` | string | - | 正常状态图片（支持 `file='path' dest='x1,y1,x2,y2' corner='x1,y1,x2,y2' fade='value'` 格式） |
| `hotimage` | string | - | 悬停状态图片 |
| `pushedimage` | string | - | 按下状态图片 |
| `disabledimage` | string | - | 禁用状态图片 |
| `focusedimage` | string | - | 获得焦点时图片 |
| `padding` | string | - | 内边距，格式：`"left,top,right,bottom"` |
| `margin` | string | - | 外边距，格式：`"left,top,right,bottom"` |
| `inset` | string | - | 内边距，格式：`"left,top,right,bottom"` |
| `textpadding` | string | - | 文本内边距，格式：`"left,top,right,bottom"` |
| `align` | string | - | 水平对齐：`"left"` \| `"center"` \| `"right"` |
| `valign` | string | - | 垂直对齐：`"top"` \| `"center"` \| `"vcenter"` \| `"bottom"` |
| `borderround` | string | - | 圆角，格式：`"x,y"` |
| `bordercolor` | string | - | 边框颜色 |
| `bordersize` | number | - | 边框大小 |
| `cursor` | string | - | 鼠标样式：`"hand"` \| `"arrow"` 等 |
| `float` | boolean | - | 是否浮动定位 |
| `pos` | string | - | 位置矩形，格式：`"x1,y1,x2,y2"` |

**内容模型：**

- 简单按钮：使用 `text` 属性即可。
- 复杂按钮：允许在 `Button` 内部声明一个 `Content` 子元素，用来定义文字、图标等内部布局。
- 当 `Button` 同时声明 `text` 和 `Content` 时，以 `Content` 为准。

**推荐写法：**

```xml
<Button min-width="80" max-width="164">
  <Content
    layout="horizontal"
    horizontal-align="right"
    vertical-align="center"
    item-spacing="4">
    <Text value="@show_more" wrap="true" />
    <Icon src="assets/arrow-down.png" width="12" height="12" />
  </Content>
</Button>
```

**按钮 ID 约定：**

| ID | 功能 |
|-----|------|
| `next` | 下一步按钮 |
| `back` | 上一步按钮 |
| `cancel` | 取消按钮 |
| `install` | 开始安装按钮 |
| `finish` | 完成按钮 |
| `browse` | 浏览文件夹按钮 |

### `action` 属性（按钮行为）

Button 元素支持 `action` 属性，用于声明式地定义按钮点击后的行为。使用 `action` 属性可以避免在代码中硬编码按钮 ID 与行为的映射关系，使 UI 逻辑完全由 XML 驱动。

```xml
<Button name="install_btn" text="立即安装" action="install" />
```

运行时，`dispatch_action`（位于 `egui_app_xml.rs`）负责解析 `action` 字符串并执行对应逻辑。

#### 可用的 action 值

| action | 说明 | 示例 |
|--------|------|------|
| `install` | 开始安装流程 | `action="install"` |
| `uninstall` | 开始卸载流程 | `action="uninstall"` |
| `launch_app` | 启动已安装的应用程序 | `action="launch_app"` |
| `close` | 直接关闭安装程序窗口 | `action="close"` |
| `close_confirm` | 显示确认对话框后关闭 | `action="close_confirm"` |
| `open_url:KEY` | 打开 `links` 配置中对应 KEY 的 URL | `action="open_url:homepage"` |
| `toggle_panel:ID:show` | 显示指定 ID 的面板 | `action="toggle_panel:options:show"` |
| `toggle_panel:ID:hide` | 隐藏指定 ID 的面板 | `action="toggle_panel:options:hide"` |
| `browse_folder` | 打开文件夹选择对话框，更新安装路径 | `action="browse_folder"` |
| `next_page` | 导航到向导的下一页 | `action="next_page"` |
| `prev_page` | 导航到向导的上一页 | `action="prev_page"` |
| `cancel` | 取消安装（等同于 `close_confirm`） | `action="cancel"` |
| `dialog_ok` | 确认对话框（关闭消息框，返回 OK） | `action="dialog_ok"` |
| `dialog_cancel` | 取消对话框（关闭消息框，返回 Cancel） | `action="dialog_cancel"` |

#### action 使用示例

**安装流程按钮：**

```xml
<!-- 配置页：展开更多选项 -->
<Button name="show_more" text="{show_more}" action="toggle_panel:options_panel:show" />

<!-- 配置页：开始安装 -->
<Button name="install_btn" text="{install_button}" action="install" />

<!-- 配置页：浏览安装路径 -->
<Button name="browse_btn" text="{browse}" action="browse_folder" />
```

**完成页按钮：**

```xml
<!-- 启动应用 -->
<Button name="launch_btn" text="{launch_button}" action="launch_app" />

<!-- 关闭安装程序 -->
<Button name="close_btn" text="{close}" action="close" />
```

**卸载流程按钮：**

```xml
<!-- 确认卸载 -->
<Button name="uninstall_btn" text="{uninstall_button}" action="uninstall" />

<!-- 取消卸载 -->
<Button name="cancel_btn" text="{not_now}" action="close" />
```

**打开外部链接：**

```xml
<!-- 打开官网（URL 在 installer_config.json 的 links 字段中定义） -->
<Button name="homepage_btn" text="官方网站" action="open_url:homepage" />

<!-- 打开用户协议 -->
<Button name="agreement_btn" text="{agreement_link}" action="open_url:agreement" />
```

对应的 `installer_config.json` 配置：

```json
{
  "links": {
    "homepage": "https://www.taptap.cn",
    "agreement": "https://www.taptap.cn/agreement"
  }
}
```

**对话框按钮：**

```xml
<!-- 消息框确认 -->
<Button name="ok_btn" text="{ok}" action="dialog_ok" />

<!-- 消息框取消 -->
<Button name="cancel_btn" text="{cancel}" action="dialog_cancel" />
```

**面板切换：**

```xml
<!-- 展开高级选项面板 -->
<Button name="expand_btn" text="{show_more}" action="toggle_panel:advanced_options:show" />

<!-- 收起高级选项面板 -->
<Button name="collapse_btn" text="{hide_more}" action="toggle_panel:advanced_options:hide" />

<!-- 被控制的面板 -->
<VerticalLayout name="advanced_options" visible="false">
  <CheckBox name="desktop_shortcut" text="{shortcut_checkbox}" selected="true" />
  <CheckBox name="autorun_checkbox" text="{autorun_checkbox}" selected="false" />
</VerticalLayout>
```

#### action 分发机制

当用户点击带有 `action` 属性的按钮时，`dispatch_action`（`egui_app_xml.rs`）按以下流程处理：

1. 从被点击按钮的 `action` 属性中读取 action 字符串
2. 解析 action 字符串（处理带参数的格式如 `open_url:KEY`、`toggle_panel:ID:show`）
3. 执行对应的操作：
   - **导航类**（`next_page`、`prev_page`）：修改向导当前页面索引
   - **流程类**（`install`、`uninstall`）：启动后台安装/卸载任务
   - **UI 类**（`toggle_panel`、`browse_folder`）：修改 UI 状态
   - **窗口类**（`close`、`close_confirm`、`launch_app`）：控制窗口生命周期
   - **对话框类**（`dialog_ok`、`dialog_cancel`）：关闭消息框并设置返回值
   - **外部类**（`open_url`）：调用系统浏览器打开 URL

如果按钮没有 `action` 属性，系统会回退到基于按钮 `name`/`id` 的默认行为映射（见上方“按钮 ID 约定”表格）。

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

#### `<Content>` - 内容容器

用于定义控件内部的内容排列，当前主要用于 `Button` 内部。

```xml
<Content
  layout="horizontal"
  horizontal-align="right"
  vertical-align="center"
  item-spacing="4">
  <Text value="@show_more" wrap="true" />
  <Icon src="assets/arrow-down.png" width="12" height="12" />
</Content>
```

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `layout` | string | `"horizontal"` | 子项排列方向：`horizontal` \| `vertical` |
| `horizontal-align` | string | `"left"` | 水平对齐：`left` \| `center` \| `right` |
| `vertical-align` | string | `"top"` | 垂直对齐：`top` \| `center` \| `bottom` |
| `item-spacing` | number | `0` | 子项之间的间距 |
| `padding` | string | - | 内容区内边距 |

#### `<Text>` - 内容文本

用于内容容器中的文本节点。

```xml
<Text value="@show_more" wrap="true" />
```

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `value` | string | 必需 | 文本内容，支持 `@key` 国际化引用 |
| `wrap` | boolean | `false` | 是否允许自动换行 |
| `color` | string | 继承父元素 | 文本颜色 |
| `font-size` | number | 继承父元素 | 字体大小 |

#### `<Icon>` - 内容图标

用于内容容器中的图标节点。

```xml
<Icon src="assets/arrow-down.png" width="12" height="12" />
```

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `src` | string | 必需 | 图标路径 |
| `width` | number | 原始宽度 | 显示宽度 |
| `height` | number | 原始高度 | 显示高度 |
| `width` | number | 可选 | 图片宽度，不指定则使用原始尺寸 |
| `height` | number | 可选 | 图片高度 |

**注意：** 系统会自动根据 DPI 加载 `@2x` 版本的图片。

#### `<Checkbox>` / `<CheckBox>` - 复选框

用户可勾选的选项。

**推荐写法：**
```xml
<Checkbox 
  id="agree_license" 
  text="我同意许可协议" 
  checked="false"
/>
```

**完整写法：**
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
| `id` / `name` | string | 必需 | 复选框标识符（`id` 与 `name` 都可使用） |
| `text` | string | 必需 | 显示文本 |
| `checked` / `selected` | boolean | false | 初始状态（`checked` 与 `selected` 都可使用） |
| `enabled` | boolean | true | 是否启用 |
| `font` | number | - | 字体 ID（引用 `<Font>` 元素） |
| `textcolor` | string | - | 文本颜色 |
| `textpadding` | string | - | 文本内边距，格式：`"left,top,right,bottom"` |
| `normalimage` | string | - | 未选中图片（支持 `file='path' dest='x1,y1,x2,y2'` 格式） |
| `normalhotimage` | string | - | 未选中悬停图片 |
| `selectedimage` | string | - | 选中图片 |
| `selectedhotimage` | string | - | 选中悬停图片 |
| `align` | string | - | 水平对齐 |
| `valign` | string | - | 垂直对齐 |

**常用 ID：**

| ID | 功能 |
|-----|------|
| `desktop_shortcut` | 创建桌面快捷方式 |
| `start_menu` | 添加到开始菜单 |
| `auto_start` | 开机自动启动 |
| `agree_license` | 同意许可协议 |

#### `<TextInput>` / `<RichEdit>` - 文本输入框

用户输入文本的控件。

**简洁写法：**
```xml
<TextInput 
  id="install_path" 
  text="C:\Program Files\MyApp" 
  width="300"
/>
```

**另一种写法：**
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
| `id` / `name` | string | 必需 | 输入框标识符（`id` 与 `name` 都可使用） |
| `text` | string | `""` | 初始文本 |
| `placeholder` | string | `""` | 占位符文本 |
| `width` | number | 200 | 宽度（像素） |
| `height` | number | 32 | 高度（像素） |
| `readonly` | boolean | false | 是否只读 |
| `multiline` | boolean | false | 是否多行 |
| `font` | number | - | 字体 ID |
| `textcolor` | string | - | 文本颜色 |
| `bkcolor` | string | - | 背景颜色 |
| `inset` | string | - | 内边距，格式：`"left,top,right,bottom"` |
| `borderround` | string | - | 圆角，格式：`"x,y"` |
| `autohscroll` | boolean | - | 自动水平滚动 |
| `wantreturn` | boolean | - | 接受回车 |
| `wantctrlreturn` | boolean | - | 接受 Ctrl+回车 |

#### `<ProgressBar>` - 进度条

显示安装进度。

**简洁写法：**
```xml
<ProgressBar 
  id="install_progress" 
  progress="0.5" 
  width="400" 
  height="8"
/>
```

**另一种写法：**
```xml
**属性：**

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `id` / `name` | string | 必需 | 进度条标识符 |
| `progress` | number | 0.0 | 进度值（0.0 ~ 1.0） |
| `min` | number | 0 | 最小值 |
| `max` | number | 100 | 最大值 |
| `value` | number | 0 | 当前值，计算进度：`(value - min) / (max - min)` |
| `width` | number | 300 | 宽度（像素） |
| `height` | number | 8 | 高度（像素） |
| `bkcolor` | string | - | 背景颜色 |
| `foreimage` | string | - | 前景图片（支持 `file='path'` 格式） |
| `thumbsize` | string | - | 前景块尺寸，格式：`"width,height"` |
| `mouse` | boolean | - | 是否可用鼠标 |
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



