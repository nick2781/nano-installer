# NSIS 布局关键字完整说明

## 一、根元素

### 1. `<Windows>`
- **说明**: 页面布局的根元素，包含所有页面内容
- **属性**: 无
- **子元素**: `<VerticalLayout>`, `<HorizontalLayout>`, `<TabLayout>`, `<Include>`

### 2. `<Window>`
- **说明**: 窗口定义（用于 msgbox 等对话框）
- **属性**:
  - `name`: 窗口名称
  - `size`: 窗口大小，格式 `"width,height"`，例如 `"800,460"`
  - `sizebox`: 窗口大小框，格式 `"left,top,right,bottom"`
  - `caption`: 标题栏，格式 `"left,top,right,bottom"`
  - `roundcorner`: 圆角，格式 `"x,y"`，例如 `"32,32"`
  - `showshadow`: 是否显示阴影，`"true"` 或 `"false"`
  - `shadowsharpness`: 阴影锐度，数值
  - `shadowsize`: 阴影大小，数值
  - `shadowpositon`: 阴影位置，格式 `"x,y"`
  - `shadowcolor`: 阴影颜色，例如 `"#000000"`
  - `shadowdarkness`: 阴影暗度，数值
  - `disabledfontcolor`: 禁用字体颜色

## 二、字体定义

### `<Font>`
- **说明**: 定义字体样式
- **属性**:
  - `id`: 字体 ID（数字），用于在其他元素中引用
  - `name`: 字体名称，例如 `"微软雅黑"`
  - `size`: 字体大小（数字）
  - `bold`: 是否粗体，`"true"` 或 `"false"`
  - `default`: 是否默认字体，`"true"` 或 `"false"`

## 三、布局容器

### 1. `<VerticalLayout>`
- **说明**: 垂直布局容器，子元素垂直排列
- **属性**:
  - `name`: 元素名称（用于脚本引用）
  - `width`: 宽度（像素）
  - `height`: 高度（像素）
  - `bkcolor`: 背景颜色，格式 `"#AARRGGBB"` 或 `"#RRGGBB"`，例如 `"#FF181B22"`
  - `bkimage`: 背景图片路径，例如 `"assets/bg_main@2x.png"`
  - `bkcolor2`: 渐变背景颜色2（可选）
  - `bkcolor3`: 渐变背景颜色3（可选）
  - `gradientangle`: 渐变角度（可选）
  - `inset`: 内边距，格式 `"left,top,right,bottom"`，例如 `"80,70,80,70"`
  - `padding`: 内边距，格式 `"left,top,right,bottom"`，例如 `"426,0,426,0"`
  - `bordercolor`: 边框颜色
  - `bordersize`: 边框大小（像素）
  - `borderround`: 边框圆角，格式 `"x,y"`，例如 `"32,32"`
  - `visible`: 是否可见，`"true"` 或 `"false"`
  - `float`: 是否浮动（绝对定位），`"true"` 或 `"false"`
  - `pos`: 浮动位置，格式 `"x1,y1,x2,y2"`（矩形区域），例如 `"0,716,1148,1036"`

### 2. `<HorizontalLayout>`
- **说明**: 水平布局容器，子元素水平排列
- **属性**: 同 `<VerticalLayout>`

### 3. `<TabLayout>`
- **说明**: 标签页布局容器
- **属性**:
  - `name`: 元素名称
- **子元素**: `<Include>`（包含其他页面）

### 4. `<Container>`
- **说明**: 容器元素，通常用作空白占位符或间距
- **属性**:
  - `name`: 元素名称
  - `width`: 宽度（像素）
  - `height`: 高度（像素）
  - `visible`: 是否可见

## 四、UI 元素

### 1. `<Button>`
- **说明**: 按钮元素
- **属性**:
  - `name`: 元素名称（用于脚本引用）
  - `text`: 按钮文本
  - `width`: 宽度（像素）
  - `height`: 高度（像素）
  - `font`: 字体 ID（引用 `<Font>` 的 `id`）
  - `textcolor`: 文本颜色，格式 `"#AARRGGBB"` 或 `"0xAARRGGBB"`，例如 `"#FFFFFFFF"` 或 `"0xffffffff"`
  - `hottextcolor`: 悬停时文本颜色
  - `pushedtextcolor`: 按下时文本颜色
  - `disabledtextcolor`: 禁用时文本颜色
  - `normalimage`: 正常状态图片，格式 `"file='path' dest='x1,y1,x2,y2' corner='x1,y1,x2,y2' fade='value'"`，例如 `"file='assets/btn_primary@2x.png'"` 或 `"file='assets/arrow-down@2x.png' dest='136,4,160,30'"`
  - `hotimage`: 悬停状态图片（格式同 `normalimage`）
  - `pushedimage`: 按下状态图片（格式同 `normalimage`）
  - `disabledimage`: 禁用状态图片（格式同 `normalimage`）
  - `focusedimage`: 获得焦点时图片（格式同 `normalimage`）
  - `padding`: 内边距，格式 `"left,top,right,bottom"`，例如 `"334,110,334,0"`
  - `margin`: 外边距，格式 `"left,top,right,bottom"`，例如 `"0,20,0,0"`
  - `inset`: 内边距（同 `padding`）
  - `textpadding`: 文本内边距，格式 `"left,top,right,bottom"`，例如 `"0,0,32,0"`
  - `align`: 水平对齐，`"left"`, `"center"`, `"right"`
  - `valign`: 垂直对齐，`"top"`, `"center"`, `"bottom"`, `"vcenter"`
  - `borderround`: 边框圆角，格式 `"x,y"`，例如 `"24,24"`
  - `cursor`: 鼠标样式，`"hand"`, `"arrow"` 等
  - `visible`: 是否可见
  - `enabled`: 是否启用

### 2. `<CheckBox>`
- **说明**: 复选框元素
- **属性**:
  - `name`: 元素名称
  - `text`: 复选框文本
  - `width`: 宽度
  - `height`: 高度
  - `font`: 字体 ID
  - `textcolor`: 文本颜色
  - `selected`: 是否选中，`"true"` 或 `"false"`
  - `normalimage`: 未选中正常状态图片，格式 `"file='path' dest='x1,y1,x2,y2'"`
  - `normalhotimage`: 未选中悬停状态图片
  - `selectedimage`: 选中正常状态图片
  - `selectedhotimage`: 选中悬停状态图片
  - `textpadding`: 文本内边距
  - `align`: 水平对齐
  - `valign`: 垂直对齐
  - `visible`: 是否可见

### 3. `<Label>`
- **说明**: 文本标签元素
- **属性**:
  - `name`: 元素名称
  - `text`: 标签文本
  - `width`: 宽度
  - `height`: 高度
  - `font`: 字体 ID
  - `textcolor`: 文本颜色
  - `textalign`: 文本对齐，`"left"`, `"center"`, `"right"`
  - `align`: 水平对齐
  - `valign`: 垂直对齐
  - `padding`: 内边距
  - `showhtml`: 是否显示 HTML，`"true"` 或 `"false"`
  - `visible`: 是否可见

### 4. `<Control>`
- **说明**: 控制元素，通常用作空白占位符或显示图片
- **属性**:
  - `name`: 元素名称
  - `width`: 宽度
  - `height`: 高度
  - `bkcolor`: 背景颜色
  - `bkimage`: 背景图片
  - `padding`: 内边距
  - `visible`: 是否可见

### 5. `<RichEdit>`
- **说明**: 富文本编辑框（文本输入框）
- **属性**:
  - `name`: 元素名称
  - `text`: 初始文本
  - `width`: 宽度
  - `height`: 高度
  - `font`: 字体 ID
  - `textcolor`: 文本颜色
  - `bkcolor`: 背景颜色
  - `inset`: 内边距
  - `borderround`: 边框圆角
  - `readonly`: 是否只读，`"true"` 或 `"false"`
  - `autohscroll`: 是否自动水平滚动，`"true"` 或 `"false"`
  - `wantreturn`: 是否接受回车，`"true"` 或 `"false"`
  - `wantctrlreturn`: 是否接受 Ctrl+回车，`"true"` 或 `"false"`
  - `multiline`: 是否多行，`"true"` 或 `"false"`
  - `visible`: 是否可见

### 6. `<Slider>`
- **说明**: 滑块元素（用于进度条）
- **属性**:
  - `name`: 元素名称
  - `width`: 宽度
  - `height`: 高度
  - `min`: 最小值（数字）
  - `max`: 最大值（数字）
  - `value`: 当前值（数字）
  - `thumbsize`: 滑块大小，格式 `"width,height"`，例如 `"12,12"`
  - `bkcolor`: 背景颜色
  - `foreimage`: 前景图片，格式 `"file='path' corner='x1,y1,x2,y2'"`
  - `mouse`: 是否可用鼠标，`"true"` 或 `"false"`
  - `enabled`: 是否启用
  - `visible`: 是否可见

## 五、其他元素

### `<Include>`
- **说明**: 包含其他布局文件
- **属性**:
  - `source`: 源文件路径，例如 `"configpage.xml"`

## 六、图片路径格式说明

### 基本格式
```
file='assets/btn_primary@2x.png'
```

### 带裁剪区域（dest）
```
file='assets/checkbox-0@2x.png' dest='0,2,32,34'
```
- `dest='x1,y1,x2,y2'`: 从图片的 `(x1,y1)` 到 `(x2,y2)` 区域裁剪

### 带圆角（corner）
```
file='assets/btn_primary.png' corner='18,6,18,14'
```
- `corner='x1,y1,x2,y2'`: 圆角参数

### 带透明度（fade）
```
file='assets/btn_close.png' fade='160'
```
- `fade='value'`: 透明度值（0-255）

### 组合格式
```
file='assets/btn_dialog@2x.png' fade='230'
file='assets/arrow-down@2x.png' dest='136,4,160,30'
file='assets/btn_primary.png' corner='18,6,18,14' fade='230'
```

## 七、颜色格式说明

### 格式 1: `#AARRGGBB`
- 例如: `"#FF181B22"` (Alpha=FF, Red=18, Green=1B, Blue=22)

### 格式 2: `#RRGGBB`
- 例如: `"#FFFFFF"` (Alpha=FF 默认)

### 格式 3: `0xAARRGGBB`
- 例如: `"0xFFFFFFFF"` (Alpha=FF, Red=FF, Green=FF, Blue=FF)

### 格式 4: `0xRRGGBB`
- 例如: `"0xffffffff"` (Alpha=FF 默认)

## 八、内边距格式说明

### `inset` 格式: `"left,top,right,bottom"`
- 例如: `inset="80,70,80,70"` (左=80, 上=70, 右=80, 下=70)

### `padding` 格式: `"left,top,right,bottom"`
- 例如: `padding="426,0,426,0"` (左=426, 上=0, 右=426, 下=0)

### `margin` 格式: `"left,top,right,bottom"`
- 例如: `margin="0,20,0,0"` (左=0, 上=20, 右=0, 下=0)

### `textpadding` 格式: `"left,top,right,bottom"`
- 例如: `textpadding="40,0,0,0"` (左=40, 上=0, 右=0, 下=0)

## 九、对齐方式说明

### `align` (水平对齐)
- `"left"`: 左对齐
- `"center"`: 居中对齐
- `"right"`: 右对齐

### `valign` (垂直对齐)
- `"top"`: 顶部对齐
- `"center"`: 居中对齐
- `"vcenter"`: 垂直居中（同 `center`）
- `"bottom"`: 底部对齐

### `textalign` (文本对齐，仅 Label)
- `"left"`: 左对齐
- `"center"`: 居中对齐
- `"right"`: 右对齐

## 十、浮动定位说明

### `float="true"`
- 启用绝对定位

### `pos="x1,y1,x2,y2"`
- 定义浮动元素的矩形区域
- `x1,y1`: 左上角坐标
- `x2,y2`: 右下角坐标
- 例如: `pos="0,716,1148,1036"` (从 (0,716) 到 (1148,1036) 的矩形)

