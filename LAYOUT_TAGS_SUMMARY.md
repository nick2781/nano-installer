# 布局标签总结（基于当前 NSIS 布局文件）

## 一、根元素

### `<Windows>`
- **说明**: 页面布局的根元素
- **属性**: 无
- **子元素**: `<VerticalLayout>`, `<HorizontalLayout>`, `<TabLayout>`, `<Include>`

### `<Window>`
- **说明**: 窗口定义（用于 msgbox 等对话框）
- **属性**: 
  - `name`, `size`, `sizebox`, `caption`, `roundcorner`, `showshadow`, `shadowsharpness`, `shadowsize`, `shadowpositon`, `shadowcolor`, `shadowdarkness`, `disabledfontcolor`
- **子元素**: `<Font>`, `<VerticalLayout>`, `<HorizontalLayout>`

## 二、字体定义

### `<Font>`
- **说明**: 定义字体样式
- **属性**: `id`, `name`, `size`, `bold`, `default`
- **位置**: 在 `<Window>` 或 `<Windows>` 根元素下

## 三、布局容器

### `<VerticalLayout>`
- **说明**: 垂直布局容器
- **属性**: `name`, `width`, `height`, `bkcolor`, `bkimage`, `bkcolor2`, `bkcolor3`, `gradientangle`, `inset`, `padding`, `bordercolor`, `bordersize`, `borderround`, `visible`, `float`, `pos`

### `<HorizontalLayout>`
- **说明**: 水平布局容器
- **属性**: 同 `<VerticalLayout>`

### `<TabLayout>`
- **说明**: 标签页布局容器
- **属性**: `name`
- **子元素**: `<Include>`

### `<Container>`
- **说明**: 容器元素（空白占位符）
- **属性**: `name`, `width`, `height`, `visible`

### `<Control>`
- **说明**: 控制元素（空白占位符，可显示图片）
- **属性**: `name`, `width`, `height`, `bkcolor`, `bkimage`, `padding`, `visible`

### `<Include>`
- **说明**: 包含其他布局文件
- **属性**: `source`

## 四、UI 元素

### `<Button>`
- **说明**: 按钮元素
- **属性**: 
  - 基础: `name`, `text`, `width`, `height`, `font`, `visible`, `enabled`
  - 颜色: `textcolor`, `hottextcolor`, `pushedtextcolor`, `disabledtextcolor`
  - 图片: `normalimage`, `hotimage`, `pushedimage`, `disabledimage`, `focusedimage`
  - 布局: `padding`, `margin`, `inset`, `textpadding`, `align`, `valign`
  - 样式: `borderround`, `cursor`

### `<CheckBox>`
- **说明**: 复选框元素
- **属性**: 
  - 基础: `name`, `text`, `width`, `height`, `font`, `textcolor`, `selected`, `visible`
  - 图片: `normalimage`, `normalhotimage`, `selectedimage`, `selectedhotimage`
  - 布局: `textpadding`, `align`, `valign`

### `<Label>`
- **说明**: 文本标签元素
- **属性**: 
  - 基础: `name`, `text`, `width`, `height`, `font`, `textcolor`, `visible`
  - 布局: `padding`, `align`, `valign`, `textalign`
  - 其他: `showhtml`

### `<RichEdit>`
- **说明**: 富文本编辑框（文本输入框）
- **属性**: 
  - 基础: `name`, `text`, `width`, `height`, `font`, `textcolor`, `bkcolor`, `visible`
  - 布局: `inset`, `borderround`
  - 功能: `readonly`, `autohscroll`, `wantreturn`, `wantctrlreturn`, `multiline`

### `<Slider>`
- **说明**: 滑块元素（用于进度条）
- **属性**: 
  - 基础: `name`, `width`, `height`, `visible`, `enabled`
  - 功能: `min`, `max`, `value`, `thumbsize`, `mouse`
  - 样式: `bkcolor`, `foreimage`

## 五、当前使用的所有标签列表

基于已读取的文件，当前使用的标签：

1. `<Windows>` - 根元素
2. `<Window>` - 窗口元素
3. `<Font>` - 字体定义
4. `<VerticalLayout>` - 垂直布局
5. `<HorizontalLayout>` - 水平布局
6. `<TabLayout>` - 标签页布局
7. `<Container>` - 容器
8. `<Control>` - 控制元素
9. `<Include>` - 包含文件
10. `<Button>` - 按钮
11. `<CheckBox>` - 复选框
12. `<Label>` - 标签
13. `<RichEdit>` - 文本输入框
14. `<Slider>` - 滑块/进度条

## 六、属性总结

### 通用属性
- `name` - 元素名称
- `width`, `height` - 尺寸
- `visible` - 可见性
- `enabled` - 启用状态
- `font` - 字体 ID

### 颜色属性
- `bkcolor` - 背景颜色
- `bkimage` - 背景图片
- `textcolor` - 文本颜色
- `bordercolor` - 边框颜色

### 布局属性
- `inset` - 内边距（left,top,right,bottom）
- `padding` - 内边距（left,top,right,bottom）
- `margin` - 外边距（left,top,right,bottom）
- `textpadding` - 文本内边距（left,top,right,bottom）

### 对齐属性
- `align` - 水平对齐（left, center, right）
- `valign` - 垂直对齐（top, center, vcenter, bottom）
- `textalign` - 文本对齐（left, center, right）

### 边框属性
- `borderround` - 圆角（x,y）
- `bordersize` - 边框大小

### 浮动定位
- `float` - 是否浮动（true/false）
- `pos` - 位置矩形（x1,y1,x2,y2）

### 图片属性
- `normalimage` - 正常状态图片
- `hotimage` - 悬停状态图片
- `pushedimage` - 按下状态图片
- `disabledimage` - 禁用状态图片
- `focusedimage` - 获得焦点图片
- `normalhotimage` - 未选中悬停图片（CheckBox）
- `selectedimage` - 选中正常图片（CheckBox）
- `selectedhotimage` - 选中悬停图片（CheckBox）
- `foreimage` - 前景图片（Slider）

### 文本属性
- `text` - 文本内容
- `showhtml` - 是否显示 HTML（Label）

### 状态属性
- `selected` - 是否选中（CheckBox）

### RichEdit 特有属性
- `readonly` - 只读
- `autohscroll` - 自动水平滚动
- `wantreturn` - 接受回车
- `wantctrlreturn` - 接受 Ctrl+回车
- `multiline` - 多行

### Slider 特有属性
- `min` - 最小值
- `max` - 最大值
- `value` - 当前值
- `thumbsize` - 滑块大小（width,height）
- `mouse` - 是否可用鼠标

### Window 特有属性
- `size` - 窗口大小（width,height）
- `sizebox` - 窗口大小框（left,top,right,bottom）
- `caption` - 标题栏（left,top,right,bottom）
- `roundcorner` - 圆角（x,y）
- `showshadow` - 显示阴影
- `shadowsharpness` - 阴影锐度
- `shadowsize` - 阴影大小
- `shadowpositon` - 阴影位置（x,y）
- `shadowcolor` - 阴影颜色
- `shadowdarkness` - 阴影暗度
- `disabledfontcolor` - 禁用字体颜色

