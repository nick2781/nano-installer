# XML 布局指南

Native runtime 当前只实现首屏所需的绝对定位子集。未列为“已支持”的元素即使能够解析，
也不会自动获得旧 eframe runtime 的行为。

页面来自 `wizard.pages`、`wizard.update_pages` 或 `wizard.uninstall_pages`；安装与卸载
开始时切到该列表的第二个页面，结束时切到最后一个页面。只有一个页面的列表会直接停在
首屏，任务结果用消息框报告。

## Page

```xml
<Page width="720" height="450"
      background-image="assets/bg_main.png"
      border-radius="12">
  ...
</Page>
```

| 属性 | 状态 | 作用 |
| --- | --- | --- |
| `width`、`height` | 已支持 | 客户区尺寸，缺省 720×450 |
| `background-image` | 已支持 | 通过 WIC 解码并铺满客户区 |
| `border-radius` | 已支持 | 使用 Win32 window region 设置圆角 |
| `background`、border 系列 | 尚未绘制 | 当前依赖背景图片 |

## 图片

```xml
<Image src="assets/logo.png" position="absolute"
       left="260" top="100" width="200" height="58" />
```

`Image` 和 `Icon` 只有在 `position="absolute"` 且包含 left/top/width/height 时才绘制。
PNG 由系统 WIC 解码为 PBGRA，再使用 GDI alpha blend 绘制。

## DPI 与图片密度

布局中的坐标和尺寸以 96 DPI 为基准。`ui.dpi_aware=true` 时，runtime 在 Windows 7 SP1+
上读取系统 DPI，并同步缩放窗口、坐标、字体、点击区域和圆角。图片路径应填写 1x 基础名：

```text
assets/logo.png
assets/logo@2x.png
```

当系统 DPI 达到 `ui.dpi_threshold`（默认 144）时优先读取 `@2x`，否则读取 1x；首选资源
不存在时自动回退到另一份。XML 中已有 `@2x` 后缀也会按当前 DPI 归一化选择，不需要维护
两套布局。

## Button

```xml
<Button action="install"
        position="absolute" left="240" top="268" width="240" height="40"
        normal-image="assets/btn_primary.png"
        text="@install_button" font-size="14" font-weight="bold"
        color="#FFFFFFFF" />
```

已支持 normal-image 和文字。`file='assets/x.png' dest='...' fade='...'` 会把图片绘制到控件内的
dest 子矩形并应用 0-255 透明度。Button 会根据鼠标进入、按下和 enabled 状态选择
hover-image、pressed-image、disabled-image，缺少状态图片时回退 normal-image。
状态变化通过离屏 GDI bitmap 完整绘制后一次提交，避免逐层重绘造成 hover 闪烁。

Button 可使用 `enabled-when="controlId:state"` 声明状态依赖。支持的 state 为
`checked`、`unchecked`、`visible`、`hidden`。条件不满足时使用 disabled-image 且不注册
点击或 hover；条件满足后恢复 normal/hover/pressed 状态。例如：

```xml
<Checkbox id="terms" checked="false" ... />
<Button action="install" enabled-when="terms:checked" ... />
```

未声明 enabled-when 的安装按钮默认可用，runtime 不包含特定协议控件名规则。复杂业务条件
应由后续脚本/UI 状态接口处理，而不是扩展控件 ID 约定。

## HBox、Content 与 Checkbox

绝对定位的 `HBox`、`VBox` 和 `Content` 会接管自己的子树：子元素不再需要坐标，由容器按
主轴依次摆放。支持固定与内容测量尺寸、`min-width`/`min-height`、`flex-grow`、
`flex-shrink`、`item-spacing`/`gap`、`padding`、`margin`（含 `margin-top` 等单边写法）、
`justify-content` 和 `align-items`。`HBox` 用 `horizontal-align`/`vertical-align` 表达
同样的对齐，`Content` 用 `layout="vertical"` 声明纵向。

内容超过可用主轴长度时，先按 flex-shrink 压缩可收缩项；`flex-shrink="0"` 的按钮保持设计
宽度。`Spacer` 只需 `flex-grow="1"`（或固定 `height`）即可吸收剩余空间。Button 内的
Content 支持 Label、Box 和 Image/Icon，用于组合文字与图标。

### 尺寸与间距

`width`/`height` 可以是像素数，也可以是相对父容器的百分比：

```xml
<VBox position="absolute" left="0" top="0" width="100%" height="100%" padding="1">
  <Spacer height="70" />
  <HBox width="100%" padding="0 32" justify-content="center">
    <Label text="@uninstall_confirm" width="100%" text-align="center" />
  </HBox>
  <Spacer flex-grow="1" />
</VBox>
```

`padding` 和 `margin` 接受 1-4 个以空格分隔的值，语义与 CSS 相同（上 右 下 左）；所有数值
以 96 DPI 为基准，随 DPI 缩放。百分比按父容器的对应边长解析。`align-items="center"` 在
交叉轴上居中：`HBox` 居中高度，`VBox` 居中宽度。

Checkbox 支持根据 `checked` 选择 checked-image/unchecked-image，绘制本地化文本，并把
Markdown 链接标记显示为 `linkcolor` 颜色的文字。未指定固定宽度时按文本内容计算宽度，
Spacer 吸收 HBox 的剩余空间；只有达到可用宽度上限时才换行。当前 checkbox 和文本链接
支持 checkbox 点击切换；文本链接的点击和打开尚未接入。

## Label 与 Select

- 绝对定位 Label：支持 `text`/`value`、font-size、font-weight、color、textalign=center。
- `action="switch_language"` 的绝对定位 Select：按当前 locale 选择匹配 Option，绘制 DPI
  对应的下拉箭头，并支持展开 XML 中声明的 Option；选择后重载对应 JSON locale 和整页文字。
- Select 的基础背景、边框和键盘操作尚未实现。

## ProgressBar

```xml
<ProgressBar id="slrProgress" position="absolute" left="72" top="326"
             width="576" height="10" progress="0" border-radius="5"
             bar-image="assets/bar_installing.png"
             background="#FF4C5868" />
```

`background` 画圆角轨道，`bar-image` 是整条渐变素材，按百分比从左向右裁剪后叠加。`progress`
是 authored 值；安装或卸载任务运行时会被实时进度覆盖，任务结束后回到 authored 值。

## 动态值绑定

TextInput 和 Label 可通过 `value-source` 绑定运行时数据：

```xml
<TextInput id="installDir" value-source="config:install.default_path" />
<Label text="@required_space"
       value-source="config:install.required_space_mb" value-format="size-mb" />
<Label text="@available_space"
       value-source="disk-free:installDir" value-format="size" />
<Label id="progress_pos" text="@installing_text" value-source="status" />
```

`config:` 后面是以点分隔的 JSON 路径。`disk-free:` 后面引用 TextInput ID；runtime 从路径
提取 Windows 卷根，再通过 `GetDiskFreeSpaceExW` 查询用户可用空间。`size-mb` 把配置中的
MB 转为可读大小，`size` 按 1024 进位格式化为 B/KB/MB/GB/TB。TextInput 当前为
只读首屏显示；目录选择对话框和用户编辑尚未接入。

`status` 用运行时发布的 locale 键替换 authored 文案，供进度页显示当前步骤；任务未运行时
保留 `text` 中的占位文案。

颜色接受 `#RRGGBB` 或 `#AARRGGBB`；当前文字绘制忽略 alpha，只使用 RGB。

## 可见性

元素自身或任一祖先包含 `visible="false"` 时不绘制。

## Actions

| action | 当前行为 |
| --- | --- |
| `minimize` | 最小化窗口 |
| `close`、`close_confirm` | 直接关闭；确认对话框尚未实现 |
| `install` | 启动安装任务，切到第二个页面并汇报进度 |
| `uninstall` | 启动卸载任务，切到第二个页面并汇报进度 |
| `launch_app` | 启动本次安装部署的 EXE，从其所在目录运行 |
| `finish` | 等价于 `close`，供完成页的退出按钮使用 |
| `switch_language` | 展开语言列表，选择后切换 locale 并重绘首屏 |
| `toggle_panel:<id>:show/hide` | 显示或隐藏目标面板，并切换对应 show/hide 控件 |

## 尚未实现的布局

- 嵌套的流式容器：容器内的子容器按固定尺寸摆放，子容器自身不再参与外层 flex 计算。
- `flex-basis`、`flex-wrap`、`align-self`、`inset`、`min-height` 之外的隐式最小尺寸。
- 参与 flex 测量的文本只按单行宽度计算；`wrap="true"` 的多行高度不参与父容器测量。
- `close_confirm` 的确认对话框、文本链接点击、目录选择框和 Select 键盘操作。
