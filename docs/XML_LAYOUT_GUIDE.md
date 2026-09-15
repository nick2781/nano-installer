# XML 布局指南

Native runtime 当前只实现首屏所需的绝对定位子集。未列为“已支持”的元素即使能够解析，
也不会自动获得旧 eframe runtime 的行为。

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

绝对定位 HBox 当前支持直接子元素的固定/内容测量宽度、`min-width`、`flex-grow`、
`flex-shrink`、`item-spacing`/`gap` 和 `align-items="center"`。内容超过可用宽度时，先按
flex-shrink 压缩可收缩项；`flex-shrink="0"` 的按钮保持设计宽度。Button 内的水平
Content 支持 Label、Box 和居中的 Image/Icon，用于组合文字与图标。

Checkbox 支持根据 `checked` 选择 checked-image/unchecked-image，绘制本地化文本，并把
Markdown 链接标记显示为 `linkcolor` 颜色的文字。未指定固定宽度时按文本内容计算宽度，
Spacer 吸收 HBox 的剩余空间；只有达到可用宽度上限时才换行。当前 checkbox 和文本链接
支持 checkbox 点击切换；文本链接的点击和打开尚未接入。

## Label 与 Select

- 绝对定位 Label：支持 `text`/`value`、font-size、font-weight、color、textalign=center。
- `action="switch_language"` 的绝对定位 Select：按当前 locale 选择匹配 Option，绘制 DPI
  对应的下拉箭头，并支持展开 XML 中声明的 Option；选择后重载对应 JSON locale 和整页文字。
- Select 的基础背景、边框和键盘操作尚未实现。

## 动态值绑定

TextInput 和 Label 可通过 `value-source` 绑定运行时数据：

```xml
<TextInput id="installDir" value-source="config:install.default_path" />
<Label text="@required_space"
       value-source="config:install.required_space_mb" value-format="size-mb" />
<Label text="@available_space"
       value-source="disk-free:installDir" value-format="size" />
```

`config:` 后面是以点分隔的 JSON 路径。`disk-free:` 后面引用 TextInput ID；runtime 从路径
提取 Windows 卷根，再通过 `GetDiskFreeSpaceExW` 查询用户可用空间。`size-mb` 把配置中的
MB 转为可读大小，`size` 按 1024 进位格式化为 B/KB/MB/GB/TB。TextInput 当前为
只读首屏显示；目录选择对话框和用户编辑尚未接入。

颜色接受 `#RRGGBB` 或 `#AARRGGBB`；当前文字绘制忽略 alpha，只使用 RGB。

## 可见性

元素自身或任一祖先包含 `visible="false"` 时不绘制。

## Actions

| action | 当前行为 |
| --- | --- |
| `minimize` | 最小化窗口 |
| `close`、`close_confirm` | 直接关闭；确认对话框尚未实现 |
| `install` | 已接入协议门禁和按钮交互状态；安装任务尚未执行 |
| `switch_language` | 展开语言列表，选择后切换 locale 并重绘首屏 |
| `toggle_panel:<id>:show/hide` | 显示或隐藏目标面板，并切换对应 show/hide 控件 |

## 尚未实现的布局

通用 VBox、嵌套 HBox、padding、inset、百分比尺寸及完整 flex 规则尚未实现。当前 HBox、
Content 和 Box 支持的是 TapTap 首屏使用的子集。
