# 页面布局

安装页取自 `wizard.pages`，卸载页取自 `wizard.uninstall_pages`。任务跑起来时向导显示列表里的
第二页，结束时切到最后一页；列表里只有一个页面时就停在这一页，用消息框报告结果。

布局以绝对定位为主，也可以使用流式容器。只有下表标着「已支持」的属性才真的生效。

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
| `background` | 已支持 | 底色，绘制在 `background-image` 之下 |
| `border-color`、`border-width` | 已支持 | 沿页面内边缘绘制描边 |

## 图片

```xml
<Image src="assets/logo.png" position="absolute"
       left="260" top="100" width="200" height="58" />
```

`Image` 与 `Icon` 只在绝对定位、并且给了 left/top/width/height 时才画出来。PNG 交给
WIC 解码成 PBGRA，再用 GDI alpha blend 绘制。

## DPI 与图片密度

布局中的坐标以 96 DPI 为基准。打开 `ui.dpi_aware` 后，运行时会读取当前显示器的 DPI，并同步缩放
窗口、坐标、字体、点击区域和圆角。安装包申请的是逐显示器感知，窗口被拖到缩放比例不同的显示器上
时会按那块显示器重新排版，不需要为不同缩放各做一套布局。

图片只填 1x 的基础名，密度交给运行时挑：

```text
assets/logo.png
assets/logo@2x.png
```

系统 DPI 达到 `ui.dpi_threshold`（默认 144）时优先使用 `@2x`，否则使用 1x；首选文件不存在时自动
回退到另一份。布局里已经写了 `@2x` 也会按当前 DPI 归一化，所以不需要维护两套布局。

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
`disabled-image`，缺少状态图时回退 `normal-image`。状态变化会先绘制到离屏位图再一次性提交，所以
鼠标悬停不会闪烁。

按钮可以依赖其他控件：

```xml
<Checkbox id="terms" checked="false" ... />
<Button action="install" enabled-when="terms:checked" ... />
```

支持的 state 为 `checked`、`unchecked`、`visible`、`hidden`，文本输入框另有 `valid` 与
`invalid`；单选组与下拉框则用它当前持着的值来写，值本身就是 state（`enabled-when="mode:custom"`）。
一条条件可以用逗号列出多个 `id:state`，全部成立按钮才可用。条件不满足时按钮使用
`disabled-image`，且不响应点击与悬停；条件满足后恢复正常状态。未声明 `enabled-when` 的按钮默认
可用，运行时不含针对特定控件名的规则。

## 流式容器

绝对定位的 `HBox`、`VBox` 与 `Content` 会接管自己的子树：子元素不再需要坐标，容器沿主轴依次摆放
它们。支持的属性包括固定与内容测量尺寸、`min-width`/`min-height`、`flex-grow`、`flex-shrink`、
`item-spacing`/`gap`、`padding`、`margin`（含 `margin-top` 等单边写法）、`justify-content` 与
`align-items`。`HBox` 也可以用 `horizontal-align`/`vertical-align` 表达对齐，`Content` 用
`layout="vertical"` 声明纵向排列。

子元素超出可用主轴空间时，先压缩可收缩项；`flex-shrink="0"` 的按钮保持设计宽度。`Spacer` 只要
`flex-grow="1"`（或固定 `height`）就能吸收剩余空间。Button 内的 Content 支持 Label、Box 与
Image/Icon，用来组合文字与图标。

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
居中：`HBox` 中居中高度，`VBox` 中居中宽度；单个子元素可以用 `align-self="start"` 或 `"end"`
覆盖容器设置。

`flex-wrap="true"`（也接受 `wrap`）在一行放不下时把元素折到下一行，窗口变窄不会把内容压扁；多行
时每一行的高度取这一行里最高的元素，`justify-content` 与 `align-self` 仍然生效。默认不折行，
放不下时先压缩可收缩项。

`flex-basis` 是分配剩余空间前元素的起始尺寸，所以 `flex-basis="0" flex-grow="1"` 会与其他弹性
元素平分剩余空间。嵌套在流式容器里的子容器会按自己的内容参与外层测量，面板不需要再写死尺寸。除
`left`/`top` 外，绝对定位元素还可以用 `right`/`bottom` 从另一侧定位，或用 `inset` 简写
（`inset="8"`，也可写 `inset-top`/`inset-right`/`inset-bottom`/`inset-left`）。

复选框按 `checked` 选择 `checked-image`/`unchecked-image`，绘制本地化文字，并把 Markdown 链接
标记显示为 `linkcolor` 颜色的文字。未指定固定宽度时按文字内容测量宽度，`HBox` 里剩下的空间交给
`Spacer` 吸收；只有达到可用宽度上限才换行。复选框可点击切换；链接可以点击打开，所以 `linkcolor`
就是把文字变成链接的开关，详见[链接](#链接)。

## 可滚动容器

绝对定位的 `HBox`、`VBox` 或 `Content` 声明 `scrollable="true"` 并给出 `id` 之后，容器保住自己拿到
的尺寸，子元素按各自声明的尺寸排开，而不是被压进这块地方。放不下的部分沿容器边缘整体裁掉：被滚过去
的那一行既不画出来，也不再登记点击，列表下方的按钮因此能接到原本会被这一行吃掉的点击。折行优先于
滚动，声明了 `flex-wrap` 的容器仍然是普通的折行容器；只写了 `scrollable` 而没有 `id` 的容器也保持
原样，因为没有名字就没有地方记住它的位置。

位置记在容器的 `id` 下，重绘和翻回这一页时都还在原地。在容器上滚动滚轮，每格移动 48 像素（随显示
缩放）；运行时为它画出的滚动条做的是同一件事：贴着尾部一条 8 像素宽的轨道，滑块长度是列表露出来的
那部分所占的比例，点轨道上滑块之外的两段就把视图挪动一页。`scrollbar-background` 与
`scrollbar-thumb-background` 可以改这两处的颜色，默认是一层半透明白。

```xml
<VBox id="components" scrollable="true" position="absolute" left="32" top="96"
      width="320" height="120" item-spacing="8">
  <Checkbox id="core" text="@component_core" width="100%" height="24" />
  <Checkbox id="shell" text="@component_shell" width="100%" height="24" />
  <Checkbox id="tools" text="@component_tools" width="100%" height="24" />
  <Checkbox id="docs" text="@component_docs" width="100%" height="24" />
  <Checkbox id="samples" text="@component_samples" width="100%" height="24" />
</VBox>
```

## Label、Select 与 RadioButton

- 绝对定位 Label：支持 `text`/`value`、font-size、font-weight、color 与 `textalign=center`。
- Select 声明自己提供的选项，在 `background` 填充与 `border-color` 描边之上绘制随 DPI 缩放的
  箭头，并显示当前选项的文字。点击控件展开选项，点中某一行就记下该行的 `value` 并收起菜单；
  声明 `visible="false"` 的选项不会出现。`action="switch_language"` 的 Select 列的是工程提供的
  语言，选中后重新加载对应语言文件并重绘页面文字。
- RadioButton 用 `group` 归属一组，用 `value` 代表组内的一个取值：`checked-image`/
  `unchecked-image` 与文字的画法和复选框一致，`checked="true"` 标出这一组起始选中的行。点击
  任意一行就把整组记为那一行的值，所以一组任何时刻只持有一个值；版面一行都没标成选中的组，
  在用户点击之前不持有任何值。
- 点中所记下的值就是页面其他部分读到的值：下拉框用自己的 `id` 作名字，单选按钮用所属 `group`
  的名字，`enabled-when="<id>:<值>"` 等的就是其中之一。
- 下拉框菜单展开后键盘接管：上下键移动高亮（首尾循环），回车记下高亮那一行——它代表的语言或
  取值——Esc 只关闭菜单，既不离开页面也不记下任何值。打开菜单时高亮停在该控件当前的值上：语言
  控件是当前语言，项目自己的选择框是用户点过的那个选项，还没点过则停在第一个选项。高亮使用
  `popup-highlight-background`，缺省回退到 `popup-selected-background`，当前值那一行用
  `popup-selected-background`。

两个记录取值的控件，以及一个要等这两者的按钮：

```xml
<RadioButton id="quick" group="mode" value="quick" text="@mode_quick" checked="true" />
<RadioButton id="custom" group="mode" value="custom" text="@mode_custom" />
<Select id="edition" background="#FF1E1E1E" border-color="#FF3A3A3A">
  <Option value="standard" text="@edition_standard" />
  <Option value="portable" text="@edition_portable" />
</Select>
<Button action="install" enabled-when="mode:custom, edition:portable" />
```

## ProgressBar

```xml
<ProgressBar id="slrProgress" position="absolute" left="72" top="326"
             width="576" height="10" progress="0" border-radius="5"
             bar-image="assets/bar_installing.png"
             background="#FF4C5868" />
```

`background` 绘制圆角轨道，`bar-image` 是整条渐变素材，按百分比从左向右裁剪后叠加。`progress`
是你写在布局里的值；安装或卸载跑起来后会拿实时进度覆盖它，结束后再恢复。

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
运行时从这个路径提取 Windows 卷根，再用 `GetDiskFreeSpaceExW` 查询可用空间。`size-mb` 把配置中的
MiB 值转成可读大小，`size` 按 1024 进位格式化为 B/KB/MB/GB/TB。

未声明 `readonly` 的 TextInput 可以直接编辑，操作方式与常见的 Windows 输入框一致：

- 点击落下闪烁光标，输入即插入文字；`Backspace`、`Delete` 删除，左右键、`Home`、`End` 移动光标。
- 按住鼠标左键拖动可以选中一段文字，选区以浅蓝高亮显示；双击选中光标下的一个词，`Ctrl+A` 全选。
- `Ctrl+C`、`Ctrl+X`、`Ctrl+V` 复制、剪切与粘贴；`Ctrl+Z` 撤销、`Ctrl+Y` 重做，连续输入会合并
  为一步撤销。
- `Ctrl+Backspace`、`Ctrl+Delete` 按词删除，`Ctrl+Left`、`Ctrl+Right` 按词移动光标。
- 有选区时直接输入或粘贴会替换掉选中内容。
- 输入法组合窗停在光标处，候选框紧贴光标下方，中日韩文字直接在输入框内完成组合。

选词规则与 Windows 一致：字母、数字和下划线组成一个词，连续空白算一段，路径分隔符等符号各自
独立。双击 `C:\Program Files` 里的反斜杠只会选中那个反斜杠。

只想展示的字段声明 `readonly="true"`，它不再响应点击与输入，`pick_directory` 也会把它当成展示
字段。用户输入的值在本次运行中优先于 `value`/`value-source` 默认值，读取同一控件的 `disk-free:`
绑定会立刻按新值重新计算。

`status` 会用运行时发布的 locale 键替换布局里的文字，进度页正是靠它显示当前步骤；任务未运行时
保留 `text` 中的占位文案。

字段还可以写明什么样的值才算数，页面上的其他控件据此行动：

```xml
<TextInput id="editDir" required="true" required-message="@dir_required"
           pattern="?:*" pattern-message="@dir_absolute" />
<Button action="install" enabled-when="chkAgree:checked, editDir:valid" />
<Label value-source="field-error:editDir" color="#FFFF7A7A" />
```

| 属性 | 作用 |
| --- | --- |
| `required="true"` | 这个字段不能为空 |
| `min-length`、`max-length` | 值至少、至多多少字，按字符算，不按字节 |
| `pattern` | 整个值要符合的掩码：`*` 是任意一串字符（可以为空），`?` 是正好一个字符，其余字符就是它本身 |
| `required-message`、`min-length-message`、`max-length-message`、`pattern-message` | 值违反这条规则时显示的 locale 键（`@键`） |

值没有违反任何一条自己声明的规则时，这个字段就是有效的；可选字段留空算有效，没写规则的字段也
有效。`enabled-when` 除了复选框和面板那四种状态，还可以对字段 id 写 `valid` 与 `invalid`。带
`value-source="field-error:<字段 id>"` 的标签显示这个字段的值最先违反的那条规则写下的文案，值合格
时什么也不画；文案和别的 `@键` 一样取自 locale 文件，漏翻会在构建时被报出来。

空的输入框照样是输入框：点进去就能落光标，否则页面要用户填的那个值根本没法填。

颜色接受 `#RRGGBB` 或 `#AARRGGBB`；当前文字绘制忽略 alpha，只使用 RGB。

## 可见性

一个元素带了 `visible="false"`，或者它的某个祖先带了，就不绘制。

## 动作

| action | 行为 |
| --- | --- |
| `minimize` | 最小化窗口 |
| `close` | 直接关闭；任务运行期间这次点击会被忽略，窗口归任务所有 |
| `close_confirm` | 先按 `ui.dialog_layout` 弹出皮肤确认框，确认后关闭；任务运行期间确认则停掉任务 |
| `pick_directory` | 打开系统目录选择框，把结果写入 TextInput，详见[目录选择](#目录选择) |
| `open_url:<键>` | 打开 `links` 表中该键配置的网址 |
| `open_url:<https://...>` | 直接打开写出的网址 |
| `install` | 启动安装任务，切到汇报它的那一页并汇报进度 |
| `uninstall` | 启动卸载任务，切到汇报它的那一页并汇报进度 |
| `launch_app` | 启动本次安装部署的 EXE，从其所在目录运行 |
| `finish` | 等同于 `close`，供完成页使用 |
| `cancel` | 停掉正在运行的任务；没有任务在跑时关闭向导 |
| `switch_language` | 展开语言列表，选择后切换 locale |
| `next` | 切到工程声明的下一页 |
| `back` | 切回工程声明的上一页 |
| `toggle_panel:<id>:show/hide` | 显示或隐藏目标面板，并切换配对的 show/hide 控件 |
| `dialog_ok` | 确认当前确认框：退出提问会停掉正在运行的任务或关闭安装程序，提示框只是收起 |
| `dialog_cancel` | 收起当前确认框，回到下面的页面 |

一个工程声明了多于一页时，就用 `next` 和 `back` 把它们走完：第一页后面没有东西，最后一页前面
也没有，所以走到头就停下，不会绕回去。任务本身在第二页汇报、在最后一页收尾；页面也可以用
`role` 自己声明职责——`progress` 标记任务汇报的那一页，`finish` 标记收尾的那一页，许可协议页
或选项页就是这样排到任务前面的。这两个职责见[文件、语言与页面](CONFIG_REFERENCE.md#文件、语言与页面)。

任务跑起来之后也可以停下：`cancel` 按钮当场就停，关闭提问回答"是"同样如此。点击不能让机器上留下
装了一半的产品，所以任务会在当前这一步走完的检查点放弃，撤回已经写下的内容，向导回到任务起始的那一页；
脚本通过 `is_cancelled()` 看到同一个请求，可以让自己的一步提前收尾，见[脚本接口](SCRIPT_API.md)。

## 对话框

确认框不是系统弹窗，而是一个普通布局：运行时把 `ui.dialog_layout` 指向的文件（默认
`layouts/msgBox.xml`）画在窗口中央，并在它和页面之间压一层遮罩。提问期间只有对话框上的控件响应
点击，页面按钮不会被误触；`Enter` 等同于确认，`Escape` 等同于取消。

对话框的文字来自当前这次提问，而不是布局里写死的文案，用 `value-source` 取值：

```xml
<Page width="400" height="180" background="#FF2A3844" border-radius="16">
  <Label id="lblMsg" value-source="dialog:message" wrap="true" width="336" />
  <Button id="btnCancel" action="dialog_cancel" value-source="dialog:dismiss"
          visible-with="dismiss" width="160" height="40" />
  <Button id="btnOK" action="dialog_ok" value-source="dialog:accept"
          width="160" height="40" />
</Page>
```

| `value-source` | 取值 |
| --- | --- |
| `dialog:message` | 本次提问或提示的正文 |
| `dialog:accept` | 确认按钮文案 |
| `dialog:dismiss` | 取消按钮文案，提示框下为空 |

`visible-with="dismiss"` 让控件只在提供两种答案时出现，于是一份布局既能当退出提问，也能当只有
一个「确定」的提示。

`Page` 的 `height` 是最小高度。对话框的文案随语言变化，同一句话在别的语言里可能多占一行；这时
卡片会按内容自动变高并保持居中，而不是把答案挤出下边缘。答案和卡片边缘之间留多少白，看布局自己
的 `padding`，示例里给底部留了 24 像素。

## 链接

`text`、`value` 或语言文件里的 `[文字](目标)` 会在元素同时声明 `linkcolor` 时变成可点击文本：

```xml
<Label text="@agree_full" linkcolor="#00C4B2" />
```

目标按以下顺序解析：绝对网址（`https://`、`http://`、`mailto:`）、`installer_config.json` 中的
`links` 表、以及历史别名 `agreement` 与 `policy`（分别映射到 `terms_of_service` 与
`privacy_policy`）。解析不到目标时保持普通文字，不会报错。

```json
"links": {
  "terms_of_service": "https://www.taptap.cn/doc/terms/",
  "privacy_policy": "https://www.taptap.cn/doc/privacy-policy/",
  "help": "https://www.taptap.cn/help"
}
```

`Button` 或其他元素也可以用 `action="open_url:help"` 走同一张表，或写作
`action="open_url:https://..."` 直接给出完整网址。

## 点击目标与光标

Button、Select、复选框与单选按钮，以及任何声明了 `action` 的元素都响应点击，鼠标悬停时变成
手型。没有 `action` 的控件即使压在可点击区域上也不响应，图标或文字正是靠这一点变成链接的。

## 目录选择

```xml
<TextInput id="editDir" value-source="config:install.default_path" cursor="text" />
<Image id="editDirIcon" action="pick_directory" target="editDir"
       cursor="hand" src="assets/folder-icon.png"
       position="absolute" left="436" top="12" width="16" height="16" />
```

`action="pick_directory"` 打开系统目录选择框。选中的路径写入 `target` 指定的 TextInput；未写
`target` 时写入页面中第一个可编辑的 TextInput，`readonly` 字段属于展示用途，只在没有别的候选时
才使用。写入的值作为用户输入保存，之后优先生效，读取同一控件的 `disk-free:` 绑定会立刻按新路径
重新计算。

## 尚未实现

- `min-width`/`min-height` 之外的隐式最小尺寸，以及流式容器的 `min-height`。
- 元素交叉轴尺寸默认仍是容器边长（未声明尺寸时），没有进一步的 `stretch`/`baseline` 区分。
- 组合窗会在焦点或光标移动时重新定位，但不会在组合过程中跟随页面滚动。
