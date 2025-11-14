# NSIS 布局格式直接实现计划

## 一、目标

**直接实现标准 XML 布局格式**（基于 NSIS 布局规范），不兼容现有格式。因为现有格式有 50% 错误且未使用，直接采用标准布局格式。

## 二、实施原则

1. **完全按照标准 XML 布局格式实现**（基于 NSIS 布局规范），不保留现有格式
2. **直接使用标准布局文件**，无需转换
3. **逐步替换现有实现**，确保功能完整

## 三、实施阶段

### 阶段 1: 核心解析器重构（优先级：最高）

#### 1.1 修改现有 XML 解析器
- **文件**: `installer/lib/src/layout/xml_parser.rs` (在当前文件基础上修改)
- **策略**: 
  - 备份现有代码（注释或创建 `.bak` 文件）
  - 逐步添加 NSIS 格式支持
  - 保留可用的现有逻辑
- **任务**:
  - [ ] 支持 `<Windows>` 作为根元素
  - [ ] 支持 `<Window>` 作为根元素（用于 msgbox）
  - [ ] 解析 `<Font>` 元素，建立字体 ID 映射表
  - [ ] 支持 `<VerticalLayout>` 和 `<HorizontalLayout>`
  - [ ] 支持 `<Container>` 和 `<Control>`（空白占位符）
  - [ ] 支持 `<Button>`, `<CheckBox>`, `<Label>`
  - [ ] 支持 `<RichEdit>`（文本输入框）
  - [ ] 支持 `<Slider>`（进度条）
  - [ ] 支持 `<TabLayout>` 和 `<Include>`（可选，后期实现）

#### 1.2 属性解析
- **标准布局属性直接解析**，不进行转换：
  - `name` → 元素 ID
  - `bkcolor` → 背景颜色
  - `bkimage` → 背景图片
  - `textcolor` → 文本颜色
  - `inset` → 内边距（格式：`"left,top,right,bottom"`）
  - `padding` → 内边距（格式：`"left,top,right,bottom"`）
  - `margin` → 外边距（格式：`"left,top,right,bottom"`）
  - `textpadding` → 文本内边距（格式：`"left,top,right,bottom"`）
  - `align` → 水平对齐
  - `valign` → 垂直对齐
  - `textalign` → 文本对齐（Label）
  - `font` → 字体 ID（需要查找字体映射表）
  - `width`, `height` → 尺寸
  - `visible`, `enabled`, `selected` → 布尔值
  - `float`, `pos` → 浮动定位
  - `borderround`, `bordercolor`, `bordersize` → 边框属性
  - 图片属性：`normalimage`, `hotimage`, `pushedimage`, `disabledimage`, `focusedimage`
  - CheckBox 特有：`normalhotimage`, `selectedimage`, `selectedhotimage`
  - RichEdit 特有：`readonly`, `autohscroll`, `wantreturn`, `wantctrlreturn`, `multiline`
  - Slider 特有：`min`, `max`, `value`, `thumbsize`, `mouse`, `foreimage`

#### 1.3 图片路径解析
- **格式**: `file='path' dest='x1,y1,x2,y2' corner='x1,y1,x2,y2' fade='value'`
- **解析逻辑**:
  ```rust
  struct ImagePath {
      file: String,           // 图片路径
      dest: Option<(u32, u32, u32, u32)>,  // 裁剪区域
      corner: Option<(u32, u32, u32, u32)>, // 圆角参数
      fade: Option<u8>,       // 透明度
  }
  ```

#### 1.4 字体系统
- **解析 `<Font>` 元素**:
  ```rust
  struct FontConfig {
      id: u32,
      name: String,
      size: f32,
      bold: bool,
      default: bool,
  }
  ```
- **建立字体映射表**: `HashMap<u32, FontConfig>`
- **字体 ID 解析**: `font="0"` → 查找映射表 → 获取字体配置

### 阶段 2: 数据结构扩展（优先级：高）

#### 2.1 扩展 `ElementAttributes`
- **文件**: `installer/lib/src/layout/element.rs`
- **策略**: 
  - 备份现有代码（注释或 `.bak` 文件）
  - 在现有结构基础上添加新字段
  - 保留现有字段（如果仍然有用）
- **任务**:
  - [ ] 添加 NSIS 标准属性字段：
    - `name: Option<String>` (元素名称)
    - `bkcolor: Option<String>` (背景颜色)
    - `bkimage: Option<String>` (背景图片)
    - `textcolor: Option<String>` (文本颜色)
    - `inset: Option<(f32, f32, f32, f32)>` (内边距，left,top,right,bottom)
    - `padding: Option<(f32, f32, f32, f32)>` (内边距，left,top,right,bottom)
    - `margin: Option<(f32, f32, f32, f32)>` (外边距，left,top,right,bottom)
    - `textpadding: Option<(f32, f32, f32, f32)>` (文本内边距，left,top,right,bottom)
    - `valign: Option<String>` (垂直对齐)
    - `textalign: Option<String>` (文本对齐)
    - `font_id: Option<u32>` (字体 ID)
    - `borderround: Option<(f32, f32)>` (圆角)
    - `bordercolor: Option<String>` (边框颜色)
    - `bordersize: Option<f32>` (边框大小)
    - `float: Option<bool>` (浮动定位)
    - `pos: Option<(f32, f32, f32, f32)>` (位置矩形，x1,y1,x2,y2)
    - 图片属性：`normalimage`, `hotimage`, `pushedimage`, `disabledimage`, `focusedimage` (类型: `Option<ImagePath>`)
    - CheckBox 特有图片属性
    - RichEdit 特有属性
    - Slider 特有属性

#### 2.2 扩展 `ElementType`
- **文件**: `installer/lib/src/layout/element.rs`
- **策略**: 
  - 备份现有代码（注释或 `.bak` 文件）
  - 在现有枚举基础上添加新变体
  - 保留现有变体（如果仍然有用）
- **任务**:
  - [ ] 添加 NSIS 标准元素类型变体：
    - `Windows` (根元素)
    - `Window` (窗口元素)
    - `VerticalLayout`
    - `HorizontalLayout`
    - `Container`
    - `Control`
    - `Button`
    - `CheckBox`
    - `Label`
    - `RichEdit`
    - `Slider`
    - `TabLayout` (可选)
    - `Include` (可选)
    - `Font` (字体定义)

### 阶段 3: 渲染器扩展（优先级：高）

#### 3.1 扩展布局渲染器
- **文件**: `installer/lib/src/ui/layout_renderer.rs`
- **策略**: 
  - 备份现有代码（注释或 `.bak` 文件）
  - 在现有方法基础上修改或添加新方法
  - 保留可用的现有逻辑
- **任务**:
  - [ ] 修改 `render_page` 方法，支持 `<Windows>` 和 `<Window>` 根元素
  - [ ] 添加 `render_vertical_layout` 方法（如果不存在）
  - [ ] 添加 `render_horizontal_layout` 方法（如果不存在）
  - [ ] 添加 `render_container` 方法（空白占位符，可显示图片）
  - [ ] 添加 `render_control` 方法（空白占位符，可显示图片）
  - [ ] 修改 `render_button` 方法，支持 NSIS 属性（`name`, `inset`, `valign`, `borderround` 等）
  - [ ] 修改 `render_checkbox` 方法，支持 NSIS 属性（`name`, `normalhotimage`, `selectedimage` 等）
  - [ ] 修改 `render_label` 方法，支持 NSIS 属性（`name`, `valign`, `textalign` 等）
  - [ ] 添加 `render_richedit` 方法（文本输入框，基于现有 `render_text_input`）
  - [ ] 添加 `render_slider` 方法（进度条，基于现有 `render_progress_bar`）

#### 3.2 属性处理
- **内边距处理**:
  - `inset` 和 `padding` 格式：`(left, top, right, bottom)`
  - 转换为 egui 的 padding 格式
- **对齐处理**:
  - `align`: 水平对齐（left, center, right）
  - `valign`: 垂直对齐（top, center, vcenter, bottom）
  - `textalign`: 文本对齐（Label 专用）
- **浮动定位**:
  - `float="true"` + `pos="x1,y1,x2,y2"` → 绝对定位矩形区域
- **边框和圆角**:
  - `borderround` → `egui::CornerRadius`
  - `bordercolor` + `bordersize` → `egui::Stroke`
- **图片处理**:
  - 解析 `ImagePath`，支持 `dest` 裁剪
  - 支持 `corner` 圆角（存储到 custom）
  - 支持 `fade` 透明度（存储到 custom）

#### 3.3 字体处理
- **字体 ID 查找**: 从字体映射表中获取字体配置
- **字体应用**: 使用字体名称、大小、粗体信息创建 `egui::FontId`

### 阶段 4: 测试和验证（优先级：高）

#### 4.1 使用标准布局文件测试
- [ ] 直接使用 `configpage2x.xml` 测试配置页面
- [ ] 直接使用 `msgbox2x.xml` 测试消息框
- [ ] 直接使用 `installingpage2x.xml` 测试安装页面
- [ ] 直接使用 `finishpage2x.xml` 测试完成页面
- [ ] 直接使用 `uninstallpage2x.xml` 测试卸载页面

#### 4.2 功能验证
- [ ] 所有元素正确渲染
- [ ] 所有属性正确应用
- [ ] 布局位置和尺寸正确
- [ ] 图片正确显示
- [ ] 文本正确显示
- [ ] 交互功能正常（按钮点击、复选框选择等）

#### 4.3 修复问题
- [ ] 修复布局问题
- [ ] 修复属性解析问题
- [ ] 修复渲染问题
- [ ] 修复交互问题

### 阶段 5: 清理和优化（优先级：中）

#### 5.1 清理代码
- [ ] 删除已备份的不兼容代码（确认新代码工作正常后）
- [ ] 删除不兼容的布局文件（确认新布局文件工作正常后）
- [ ] 清理注释掉的旧代码（可选，保留作为参考）

#### 5.2 代码优化
- [ ] 优化属性解析性能
- [ ] 优化图片路径解析性能
- [ ] 优化渲染性能

#### 5.3 文档更新
- [ ] 更新布局文件格式文档
- [ ] 更新开发文档
- [ ] 更新示例文件

## 四、详细实施步骤

### 步骤 1: 修改现有解析器

```rust
// installer/lib/src/layout/xml_parser.rs (在当前文件基础上修改)

// 在现有 XmlParser 结构体中添加字体映射表
pub struct XmlParser {
    // ... 现有字段 ...
    font_map: HashMap<u32, FontConfig>,  // 新增字段
}

pub struct FontConfig {
    pub id: u32,
    pub name: String,
    pub size: f32,
    pub bold: bool,
    pub default: bool,
}

pub struct ImagePath {
    pub file: String,
    pub dest: Option<(u32, u32, u32, u32)>,
    pub corner: Option<(u32, u32, u32, u32)>,
    pub fade: Option<u8>,
}

impl XmlParser {
    pub fn new() -> Self {
        Self {
            // ... 现有字段初始化 ...
            font_map: HashMap::new(),  // 新增字段
        }
    }
    
    pub fn parse(&mut self, xml_content: &str) -> Result<LayoutTree, ParseError> {
        // 1. 解析 XML
        // 2. 解析 <Font> 元素，建立映射表
        // 3. 解析布局元素
        // 4. 返回 LayoutTree
    }
    
    fn parse_font(&mut self, node: &roxmltree::Node) -> Result<FontConfig, ParseError> {
        // 解析 <Font> 元素
    }
    
    fn parse_image_path(&self, value: &str) -> ImagePath {
        // 解析图片路径格式
    }
    
    fn parse_inset(&self, value: &str) -> Result<(f32, f32, f32, f32), ParseError> {
        // 解析 inset="left,top,right,bottom"
    }
    
    fn parse_pos(&self, value: &str) -> Result<(f32, f32, f32, f32), ParseError> {
        // 解析 pos="x1,y1,x2,y2"
    }
}
```

### 步骤 2: 扩展 ElementAttributes

```rust
// installer/lib/src/layout/element.rs
// 在现有 ElementAttributes 结构体中添加新字段

#[derive(Debug, Clone, PartialEq)]
pub struct ElementAttributes {
    // ... 保留现有字段 ...
    
    // 新增：NSIS 标准属性
    pub name: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub bkcolor: Option<String>,
    pub bkimage: Option<String>,
    pub textcolor: Option<String>,
    pub inset: Option<(f32, f32, f32, f32)>,  // left, top, right, bottom
    pub padding: Option<(f32, f32, f32, f32)>, // left, top, right, bottom
    pub margin: Option<(f32, f32, f32, f32)>,  // left, top, right, bottom
    pub textpadding: Option<(f32, f32, f32, f32)>, // left, top, right, bottom
    pub align: Option<String>,  // left, center, right
    pub valign: Option<String>, // top, center, vcenter, bottom
    pub textalign: Option<String>, // left, center, right (Label only)
    pub font_id: Option<u32>,
    pub text: Option<String>,
    pub visible: Option<bool>,
    pub enabled: Option<bool>,
    pub selected: Option<bool>,
    pub borderround: Option<(f32, f32)>,
    pub bordercolor: Option<String>,
    pub bordersize: Option<f32>,
    pub float: Option<bool>,
    pub pos: Option<(f32, f32, f32, f32)>, // x1, y1, x2, y2
    
    // 图片属性
    pub normalimage: Option<ImagePath>,
    pub hotimage: Option<ImagePath>,
    pub pushedimage: Option<ImagePath>,
    pub disabledimage: Option<ImagePath>,
    pub focusedimage: Option<ImagePath>,
    
    // CheckBox 特有
    pub normalhotimage: Option<ImagePath>,
    pub selectedimage: Option<ImagePath>,
    pub selectedhotimage: Option<ImagePath>,
    
    // RichEdit 特有
    pub readonly: Option<bool>,
    pub autohscroll: Option<bool>,
    pub wantreturn: Option<bool>,
    pub wantctrlreturn: Option<bool>,
    pub multiline: Option<bool>,
    
    // Slider 特有
    pub min: Option<f32>,
    pub max: Option<f32>,
    pub value: Option<f32>,
    pub thumbsize: Option<(f32, f32)>,
    pub mouse: Option<bool>,
    pub foreimage: Option<ImagePath>,
    
    // 其他自定义属性（用于存储 corner, fade 等）
    pub custom: HashMap<String, String>,
}
```

### 步骤 3: 扩展渲染器

```rust
// installer/lib/src/ui/layout_renderer.rs
// 在现有 LayoutRenderer 实现中添加或修改方法

impl LayoutRenderer {
    // 修改现有方法或添加新方法
    
    fn render_vertical_layout(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        // 处理 bkcolor, bkimage, inset, padding 等属性
        // 垂直排列子元素
        // 可以基于现有的 render_page 或 render_vbox 逻辑
    }
    
    fn render_horizontal_layout(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        // 处理 bkcolor, bkimage, inset, padding 等属性
        // 水平排列子元素
        // 可以基于现有的 render_hbox 逻辑
    }
    
    fn render_container(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        // 空白占位符，可显示 bkimage
        // 可以基于现有的 render_spacer 逻辑
    }
    
    fn render_control(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        // 空白占位符，可显示 bkimage
        // 可以基于现有的 render_spacer 逻辑
    }
    
    fn render_richedit(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        // 文本输入框，支持 readonly, multiline 等属性
        // 可以基于现有的 render_text_input 逻辑
    }
    
    fn render_slider(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        // 进度条，支持 min, max, value, foreimage 等属性
        // 可以基于现有的 render_progress_bar 逻辑
    }
}
```

## 五、实施时间表

### 第 1 周：解析器重构
- [ ] 备份现有 `xml_parser.rs` 代码（注释或 `.bak` 文件）
- [ ] 在现有 `XmlParser` 中添加字体映射表字段
- [ ] 实现字体解析和映射（解析 `<Font>` 元素）
- [ ] 实现图片路径解析（支持 `file`, `dest`, `corner`, `fade`）
- [ ] 扩展属性解析（支持 `inset`, `padding`, `pos`, `valign` 等）
- [ ] 扩展元素解析（支持 `<VerticalLayout>`, `<HorizontalLayout>`, `<Container>`, `<Control>`, `<CheckBox>`, `<RichEdit>`, `<Slider>` 等）

### 第 2 周：数据结构重构
- [ ] 重写 `ElementAttributes`
- [ ] 重写 `ElementType`
- [ ] 更新 `LayoutTree` 结构
- [ ] 集成 `XmlLayoutParser` 到现有系统

### 第 3 周：渲染器重构
- [ ] 重写布局容器渲染（VerticalLayout, HorizontalLayout）
- [ ] 重写 UI 元素渲染（Button, CheckBox, Label）
- [ ] 实现新元素渲染（Container, Control, RichEdit, Slider）
- [ ] 实现属性处理（对齐、边框、圆角等）

### 第 4 周：测试和优化
- [ ] 使用 NSIS 布局文件测试
- [ ] 修复所有问题
- [ ] 性能优化
- [ ] 删除旧代码
- [ ] 文档更新

## 六、关键决策

1. **不保留现有格式**：直接实现标准布局格式，不兼容现有格式
2. **逐步替换**：先实现解析器，再更新渲染器，最后测试
3. **直接使用标准布局文件**：测试时直接使用标准布局文件
4. **完整实现**：实现所有标准布局格式支持的属性和元素

## 七、风险控制

1. **解析错误**：充分测试所有标准布局文件
2. **渲染问题**：逐步实现，每个元素单独测试
3. **性能问题**：优化解析和渲染性能
4. **兼容性问题**：不兼容现有格式，但确保标准布局格式完全支持

## 八、总结

采用**直接实现标准布局格式**的方案，不保留现有格式兼容性。分 4 周完成：
1. 第 1 周：解析器重构
2. 第 2 周：数据结构重构
3. 第 3 周：渲染器重构
4. 第 4 周：测试和优化

最终目标：**完全支持标准布局格式，可以直接使用标准布局文件**。

