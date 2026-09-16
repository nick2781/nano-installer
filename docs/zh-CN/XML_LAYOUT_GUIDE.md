# 页面布局

安装页来自 `wizard.pages`，卸载页来自 `wizard.uninstall_pages`。任务执行期间向导显示列表中的
第二页，结束时切到最后一页；只有一个页面的列表会停留在该页，用消息框报告结果。

布局以绝对定位为主，也可以使用流式容器。只有下表标注「已支持」的属性会真正生效。

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
| `width`、`height` | 已支持 | 客户区尺寸，默认 720x450 |
| `background-image` | 已支持 | 由 WIC 解码并铺满客户区 |
| `border-radius` | 已支持 | 通过 Win32 region 设置窗口圆角 |
| `background` 与边框类属性 | 尚未绘制 | 当前请使用背景图 |

## 图片

```xml
<Image src="assets/logo.png" position="absolute"
       left="260" top="100" width="200" height="58" />
```

`Image` 与 `Icon` 只有在绝对定位并给出 left/top/width/height 时才绘制。PNG 由 WIC 解码为
PBGRA，再用 GDI alpha blend 绘制。

## DPI 与图片密度

布局中的坐标以 96 DPI 为基准。启用 `ui.dpi_aware` 后，运行时会读取系统 DPI，并同步缩放窗口、
坐标、字体、点击区域和圆角。

图片只填 1x 基础名，由运行时挑选密度：

```text
assets/logo.png
assets/logo@2x.png
```

系统 DPI 达到 `ui.dpi_threshold`（默认 144）时优先使用 `@2x`，否则使用 1x；首选文件不存在时
自动回退到另一份。布局里已经写了 `@2x` 也会按当前 DPI 归一化，因此不需要维护两套布局。

## Button

```xml
<Button action="install"
        position="absolute" left="240" top="268" width="240" height="40"
        normal-image="assets/btn_primary.png"
        text="@install_button" font-size="14" font-weight="bold"
        color="#FFFFFFFF" />
```

支持的属性包括 `normal-image` 与文字。`file='assets/x.png' dest='...' fade='...'` 形式会把图片绘制
到控件内的子矩形，并应用 0-255 透明度。按钮按状态选择 `hover-image`、`pressed-image` 和
`disabled-image`，缺少状态图时回退 `normal-image`。状态变化会先绘制到离屏位图再一次性提交，
因此鼠标悬停不会闪烁。

按钮可以依赖其他控件：

```xml
<Checkbox id="terms" checked="false" ... />
<Button action="install" enabled-when="terms:checked" ... />
```

支持的 state 为 `checked`、`unchecked`、`visible`、`hidden`。条件不满足时按钮使用
`disabled-image`，且不响应点击与悬停；条件满足后恢复正常状态。未声明 `enabled-when` 的按钮默认
可用，运行时不含针对特定控件名的规则。

## 流式容器

绝对定位的 `HBox`、`VBox` 与 `Content` 会接管自己的子树：子元素不再需要坐标，由容器沿主轴依次
摆放。支持的属性包括固定与内容测量尺寸、`min-width`/`min-height`、`flex-grow`、`flex-shrink`、
`item-spacing`/`gap`、`padding`、`margin`（含 `margin-top` 等单边写法）、`justify-content` 与
`align-items`。`HBox` 也可以用 `horizontal-align`/`vertical-align` 表达对齐，`Content` 用
`layout="vertical"` 声明纵向排列。

子元素超出可用主轴空间时，先压缩可收缩项；`flex-shrink="0"` 的按钮保持设计宽度。`Spacer` 只要
`flex-grow="1"`（或固定 `height`）就能吸收剩余空间。Button 内的 Content 支持 Label、Box 与
Image/Icon，用于组合文字与图标。

### 尺寸与间距

`width`、`height` 可以是像素，也可以是相对父容器的百分比：

```xml
<VBox position="absolute" left="0" top="0" width="100%" height="100%" padding="1">
  <Spacer height="70" />
  <HBox width="100%" padding="0 32" justify-content="center">
    <Label text="@uninstall_confirm" width="100%" text-align="center" />
  </HBox>
  <Spacer flex-grow="1" />
</VBox>
```

`padding` 与 `margin` 接受 1-4 个以空格分隔的值，语义与 CSS 相同（上 右 下 左）。所有数值以
96 DPI 为基准，随系统 DPI 缩放；百分比按父容器的对应边长解析。`align-items="center"` 在交叉轴上
居中：`HBox` 中居中高度，`VBox` 中居中宽度。

复选框按 `checked` 选择 `checked-image`/`unchecked-image`，绘制本地化文字，并把 Markdown 链接
标记显示为 `linkcolor` 颜色的文字。未指定固定宽度时按文字内容测量宽度，由 `Spacer` 吸收 `HBox`
的剩余空间；只有达到可用宽度上限才换行。当前复选框可以点击切换，链接点击尚未接入。

## Label 与 Select

- 绝对定位 Label：支持 `text`/`value`、font-size、font-weight、color 与 `textalign=center`。
- `action="switch_language"` 的 Select：按当前 locale 选中对应 Option，绘制随 DPI 缩放的箭头，
  并展开 XML 中声明的选项。选择后重新加载对应语言文件并重绘页面文字。
- Select 的背景、边框与键盘操作尚未实现。

## ProgressBar

```xml
<ProgressBar id="slrProgress" position="absolute" left="72" top="326"
             width="576" height="10" progress="0" border-radius="5"
             bar-image="assets/bar_installing.png"
             background="#FF4C5868" />
```

`background` 绘制圆角轨道，`bar-image` 是整条渐变素材，按百分比从左向右裁剪后叠加。`progress`
是布局里写的值；安装或卸载运行时会被实时进度覆盖，结束后恢复。

## 动态值绑定

TextInput 与 Label 可以通过 `value-source` 绑定运行时数据：

```xml
<TextInput id="installDir" value-source="config:install.default_path" />
<Label text="@required_space"
       value-source="config:install.required_space_mb" value-format="size-mb" />
<Label text="@available_space"
       value-source="disk-free:installDir" value-format="size" />
<Label id="progress_pos" text="@installing_text" value-source="status" />
```

`config:` 后面是以点分隔的 `installer_config.json` 路径。`disk-free:` 后面写 TextInput 的 id；
运行时从该路径提取 Windows 卷根，再用 `GetDiskFreeSpaceExW` 查询可用空间。`size-mb` 把配置中的
MiB 值转成可读大小，`size` 按 1024 进位格式化为 B/KB/MB/GB/TB。TextInput 目前是只读展示，
目录选择和用户编辑尚未接入。

`status` 会用运行时发布的 locale 键替换布局里的文字，进度页正是靠它显示当前步骤；任务未运行时
保留 `text` 中的占位文案。

颜色接受 `#RRGGBB` 或 `#AARRGGBB`；当前文字绘制忽略 alpha，只使用 RGB。

## 可见性

元素自身或任一祖先带 `visible="false"` 时不绘制。

## 动作

| action | 行为 |
| --- | --- |
| `minimize` | 最小化窗口 |
| `close`、`close_confirm` | 直接关闭；确认对话框尚未实现 |
| `install` | 启动安装任务，切到第二页并汇报进度 |
| `uninstall` | 启动卸载任务，切到第二页并汇报进度 |
| `launch_app` | 启动本次安装部署的 EXE，从其所在目录运行 |
| `finish` | 等同于 `close`，供完成页使用 |
| `switch_language` | 展开语言列表，选择后切换 locale |
| `toggle_panel:<id>:show/hide` | 显示或隐藏目标面板，并切换配对的 show/hide 控件 |

## 尚未实现

- 嵌套流式布局：内层容器保持固定尺寸，不参与外层的 flex 计算。
- `flex-basis`、`flex-wrap`、`align-self`、`inset`，以及 `min-height` 之外的隐式最小尺寸。
- 参与 flex 测量的文字只按单行宽度计算；`wrap="true"` 的多行高度不参与父容器测量。
- `close_confirm` 确认对话框、链接点击、目录选择框以及 Select 的键盘操作。
