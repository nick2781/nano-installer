# NSIS 布局格式完全兼容迁移方案

## 一、目标

让我们的布局系统**完全兼容** NSIS 的布局格式，可以直接使用 NSIS 的 XML 布局文件，无需任何转换。

## 二、方案选择

### 推荐方案：完全兼容（双向支持）

**优点**:
- ✅ 可以直接使用 NSIS 的布局文件
- ✅ 保持向后兼容，现有布局文件仍然可用
- ✅ 与 NSIS 完全一致，便于参考和调试
- ✅ 无需维护两套布局文件

**实施策略**:
- 扩展解析器，同时支持 NSIS 格式和当前格式
- 在解析时进行格式转换和属性映射
- 保持渲染器不变，只修改解析层

## 三、详细实施计划

### 阶段 1: 根元素和基础容器（优先级：高）

#### 1.1 支持 `<Windows>` 根元素
- **文件**: `installer/lib/src/layout/xml_parser.rs`
- **修改**:
  - 允许 `<Windows>` 作为根元素（可选，兼容 `<Layout>`）
  - `<Windows>` 下直接包含布局容器，无需 `<Page>` 包装

#### 1.2 支持 NSIS 布局容器
- **元素映射**:
  - `<VerticalLayout>` → `ElementType::VBox`
  - `<HorizontalLayout>` → `ElementType::HBox`
  - `<Container>` → `ElementType::Spacer`（空白占位符）
  - `<Control>` → `ElementType::Spacer`（空白占位符，可显示图片）

#### 1.3 支持 `<TabLayout>` 和 `<Include>`
- `<TabLayout>`: 标签页容器（暂时映射为普通容器）
- `<Include>`: 包含其他布局文件（需要实现文件包含逻辑）

### 阶段 2: UI 元素映射（优先级：高）

#### 2.1 元素类型映射
- `<Button>` → `ElementType::Button`（已支持，需扩展属性）
- `<CheckBox>` → `ElementType::Checkbox`（需支持，当前是 `<Checkbox>`）
- `<Label>` → `ElementType::Label`（已支持，需扩展属性）
- `<RichEdit>` → `ElementType::TextInput`（需支持，当前是 `<TextInput>`）
- `<Slider>` → `ElementType::ProgressBar`（需支持，当前是 `<ProgressBar>`）

#### 2.2 元素命名兼容
- 同时支持 `<CheckBox>` 和 `<Checkbox>`
- 同时支持 `<RichEdit>` 和 `<TextInput>`
- 同时支持 `<Slider>` 和 `<ProgressBar>`

### 阶段 3: 属性映射和转换（优先级：高）

#### 3.1 基础属性映射

| NSIS 属性 | 当前属性 | 转换逻辑 |
|-----------|----------|----------|
| `name` | `id` | 直接映射 |
| `bkcolor` | `background` (颜色) | 解析颜色值，存储到 `background` |
| `bkimage` | `background` (图片) | 解析图片路径，存储到 `background` |
| `textcolor` | `color` | 直接映射 |
| `visible` | `visible` | 直接映射（布尔值） |
| `enabled` | `enabled` | 直接映射（布尔值） |
| `selected` | `selected` | 直接映射（布尔值） |
| `width` | `width` | 直接映射（浮点数） |
| `height` | `height` | 直接映射（浮点数） |

#### 3.2 内边距格式转换

**`inset` 格式转换**:
- NSIS: `inset="left,top,right,bottom"` (例如: `"80,70,80,70"`)
- 当前: `padding="top,right,bottom,left"` (例如: `"70,80,70,80"`)
- **转换**: `inset="l,t,r,b"` → `padding="t,r,b,l"`

**`padding` 格式**:
- NSIS: `padding="left,top,right,bottom"` (例如: `"426,0,426,0"`)
- 当前: `padding="top,right,bottom,left"` (例如: `"0,426,0,426"`)
- **转换**: `padding="l,t,r,b"` → `padding="t,r,b,l"`

**`margin` 格式**:
- NSIS: `margin="left,top,right,bottom"` (例如: `"0,20,0,0"`)
- 当前: 无对应属性，存储到 `custom` 中

**`textpadding` 格式**:
- NSIS: `textpadding="left,top,right,bottom"` (例如: `"40,0,0,0"`)
- 当前: `textpadding="left,top,right,bottom"` (格式相同，直接映射)

#### 3.3 对齐属性

**`align` (水平对齐)**:
- NSIS: `"left"`, `"center"`, `"right"`
- 当前: `"left"`, `"center"`, `"right"`
- **直接映射**

**`valign` (垂直对齐)**:
- NSIS: `"top"`, `"center"`, `"vcenter"`, `"bottom"`
- 当前: 无对应属性
- **新增**: 添加到 `ElementAttributes` 或存储到 `custom` 中

**`textalign` (文本对齐，仅 Label)**:
- NSIS: `"left"`, `"center"`, `"right"`
- 当前: `align` 属性
- **映射**: `textalign` → `align` (仅对 Label 元素)

#### 3.4 图片属性

**图片路径格式解析**:
```
file='assets/btn_primary@2x.png'
file='assets/checkbox-0@2x.png' dest='0,2,32,34'
file='assets/btn_close.png' fade='160'
file='assets/btn_primary.png' corner='18,6,18,14' fade='230'
```

**解析逻辑**:
1. 提取 `file='...'` → 图片路径
2. 提取 `dest='x1,y1,x2,y2'` → 裁剪区域
3. 提取 `corner='x1,y1,x2,y2'` → 圆角参数（存储到 `custom`）
4. 提取 `fade='value'` → 透明度（存储到 `custom`）

**图片属性映射**:
- `normalimage` → `normalimage` (已支持)
- `hotimage` → `hotimage` (已支持)
- `pushedimage` → `pushedimage` (已支持)
- `disabledimage` → `disabledimage` (已支持)
- `focusedimage` → 存储到 `custom` (可选)
- `normalhotimage` → 存储到 `custom` (CheckBox 特有)
- `selectedimage` → 存储到 `custom` (CheckBox 特有)
- `selectedhotimage` → 存储到 `custom` (CheckBox 特有)

#### 3.5 字体属性

**`font` 属性**:
- NSIS: `font="0"` (字体 ID，引用 `<Font>` 元素)
- 当前: `font_size="28"` (字体大小，数字)
- **解决方案**:
  1. 解析 `<Font>` 元素，建立 ID → 字体配置映射表
  2. 在解析 `font="id"` 时，查找映射表，转换为字体大小
  3. 存储字体配置到 `custom` 中（名称、大小、粗体等）

**字体配置结构**:
```rust
struct FontConfig {
    id: u32,
    name: String,
    size: f32,
    bold: bool,
}
```

#### 3.6 边框和圆角

**`borderround`**:
- NSIS: `borderround="24,24"` (格式: `"x,y"`)
- 当前: 无对应属性
- **新增**: 存储到 `custom` 中，渲染时使用

**`bordercolor`**:
- NSIS: `bordercolor="#FF474B59"`
- 当前: 无对应属性
- **新增**: 存储到 `custom` 中

**`bordersize`**:
- NSIS: `bordersize="2"`
- 当前: 无对应属性
- **新增**: 存储到 `custom` 中

#### 3.7 浮动定位

**`float` 和 `pos`**:
- NSIS: `float="true"` + `pos="x1,y1,x2,y2"` (矩形区域)
- 当前: `position="x,y"` (点坐标)
- **转换**:
  - `float="true"` → 启用绝对定位
  - `pos="x1,y1,x2,y2"` → 计算为 `position="x1,y1"` 和 `width="x2-x1"`, `height="y2-y1"`

#### 3.8 其他属性

**RichEdit 特有属性**:
- `readonly` → 存储到 `custom` 中
- `autohscroll` → 存储到 `custom` 中
- `wantreturn` → 存储到 `custom` 中
- `wantctrlreturn` → 存储到 `custom` 中
- `multiline` → 存储到 `custom` 中

**Slider 特有属性**:
- `min` → 存储到 `custom` 中
- `max` → 存储到 `custom` 中
- `value` → 映射到 `progress` (0.0-1.0)
- `thumbsize` → 存储到 `custom` 中
- `mouse` → 存储到 `custom` 中
- `foreimage` → 存储到 `custom` 中

**其他**:
- `cursor` → 存储到 `custom` 中
- `showhtml` → 存储到 `custom` 中
- `heigh` → 修正为 `height` (NSIS 拼写错误)

### 阶段 4: 字体系统（优先级：中）

#### 4.1 解析 `<Font>` 元素
- 在 `<Window>` 或 `<Windows>` 根元素下查找 `<Font>` 子元素
- 建立字体 ID → 字体配置映射表
- 存储到解析器上下文中

#### 4.2 字体 ID 到字体大小转换
- 解析 `font="id"` 时，查找映射表
- 转换为 `font_size` 属性
- 保留字体名称和粗体信息到 `custom` 中

### 阶段 5: 窗口定义（优先级：低）

#### 5.1 支持 `<Window>` 元素
- 解析窗口属性（`size`, `roundcorner`, `showshadow` 等）
- 存储到布局树中（可选，主要用于 msgbox）

#### 5.2 窗口属性映射
- `size` → 窗口大小
- `roundcorner` → 圆角参数
- `showshadow` → 阴影设置
- 其他属性存储到 `custom` 中

### 阶段 6: 渲染器扩展（优先级：中）

#### 6.1 支持 `valign` 属性
- 在渲染时处理垂直对齐
- 影响文本和元素的垂直位置

#### 6.2 支持 `borderround` 属性
- 在渲染按钮、容器等元素时应用圆角
- 使用 `egui::CornerRadius`

#### 6.3 支持 `textpadding` 的完整格式
- 当前已支持，确保正确应用

#### 6.4 支持 `pos` 格式的浮动定位
- 处理矩形区域的绝对定位
- 计算正确的宽度和高度

#### 6.5 支持 RichEdit 特有属性
- `readonly`: 禁用文本输入
- `multiline`: 多行文本输入

#### 6.6 支持 Slider 渲染
- 将 `Slider` 渲染为进度条
- 处理 `min`, `max`, `value` 属性
- 支持前景图片

### 阶段 7: 测试和验证（优先级：高）

#### 7.1 直接使用 NSIS 布局文件测试
- 使用 `configpage2x.xml` 作为测试
- 使用 `msgbox2x.xml` 作为测试
- 使用其他页面布局文件测试

#### 7.2 验证所有元素和属性
- 确保所有元素正确渲染
- 确保所有属性正确应用
- 修复兼容性问题

#### 7.3 性能优化
- 优化属性映射性能
- 优化图片路径解析性能

## 四、实施细节

### 4.1 属性映射函数

```rust
// 在 xml_parser.rs 中
fn map_nsis_attribute(key: &str, value: &str, element_type: &ElementType) -> (String, String) {
    match key {
        // 基础属性
        "name" => ("id".to_string(), value.to_string()),
        "bkcolor" => ("background".to_string(), value.to_string()),
        "bkimage" => ("background".to_string(), value.to_string()),
        "textcolor" => ("color".to_string(), value.to_string()),
        
        // 内边距格式转换
        "inset" => {
            // "left,top,right,bottom" -> "top,right,bottom,left"
            let parts: Vec<&str> = value.split(',').collect();
            if parts.len() == 4 {
                let padding = format!("{},{},{},{}", parts[1], parts[2], parts[3], parts[0]);
                ("padding".to_string(), padding)
            } else {
                (key.to_string(), value.to_string())
            }
        },
        "padding" => {
            // NSIS: "left,top,right,bottom" -> 当前: "top,right,bottom,left"
            let parts: Vec<&str> = value.split(',').collect();
            if parts.len() == 4 {
                let padding = format!("{},{},{},{}", parts[1], parts[2], parts[3], parts[0]);
                ("padding".to_string(), padding)
            } else {
                (key.to_string(), value.to_string())
            }
        },
        
        // 字体 ID 转换
        "font" => {
            // 查找字体映射表，转换为 font_size
            if let Some(font_config) = self.font_map.get(&value.parse::<u32>().ok()?) {
                ("font_size".to_string(), font_config.size.to_string())
            } else {
                (key.to_string(), value.to_string())
            }
        },
        
        // 浮动定位
        "pos" => {
            // "x1,y1,x2,y2" -> position="x1,y1" + width="x2-x1" + height="y2-y1"
            let parts: Vec<&str> = value.split(',').collect();
            if parts.len() == 4 {
                let x1: f32 = parts[0].parse().unwrap_or(0.0);
                let y1: f32 = parts[1].parse().unwrap_or(0.0);
                let x2: f32 = parts[2].parse().unwrap_or(0.0);
                let y2: f32 = parts[3].parse().unwrap_or(0.0);
                // 返回 position，width 和 height 需要单独处理
                ("position".to_string(), format!("{},{}", x1, y1))
            } else {
                (key.to_string(), value.to_string())
            }
        },
        
        // 其他属性直接映射或存储到 custom
        _ => (key.to_string(), value.to_string()),
    }
}
```

### 4.2 元素类型映射函数

```rust
fn map_nsis_element_type(name: &str) -> Result<ElementType, ParseError> {
    match name {
        "Windows" => Ok(ElementType::Page), // 根元素，映射为 Page
        "Window" => Ok(ElementType::Page),  // 窗口元素，映射为 Page
        "VerticalLayout" => Ok(ElementType::VBox),
        "HorizontalLayout" => Ok(ElementType::HBox),
        "Container" => Ok(ElementType::Spacer),
        "Control" => Ok(ElementType::Spacer),
        "Button" => Ok(ElementType::Button),
        "CheckBox" => Ok(ElementType::Checkbox),
        "Label" => Ok(ElementType::Label),
        "RichEdit" => Ok(ElementType::TextInput),
        "Slider" => Ok(ElementType::ProgressBar),
        "TabLayout" => Ok(ElementType::VBox), // 暂时映射为 VBox
        "Include" => Err(ParseError::InvalidElementType("Include 需要特殊处理".to_string())),
        "Font" => Err(ParseError::InvalidElementType("Font 需要特殊处理".to_string())),
        _ => {
            // 回退到现有解析逻辑（支持当前格式）
            self.parse_element_type(name)
        }
    }
}
```

### 4.3 图片路径解析函数

```rust
fn parse_image_path(value: &str) -> ImagePath {
    // 解析格式: file='path' dest='x1,y1,x2,y2' corner='x1,y1,x2,y2' fade='value'
    let mut path = String::new();
    let mut dest = None;
    let mut corner = None;
    let mut fade = None;
    
    // 使用正则表达式或字符串解析
    // 提取 file='...'
    if let Some(start) = value.find("file='") {
        let end = value[start+6..].find("'").unwrap_or(value.len());
        path = value[start+6..start+6+end].to_string();
    }
    
    // 提取 dest='...'
    if let Some(start) = value.find("dest='") {
        let end = value[start+6..].find("'").unwrap_or(value.len());
        let dest_str = &value[start+6..start+6+end];
        let parts: Vec<&str> = dest_str.split(',').collect();
        if parts.len() == 4 {
            dest = Some((
                parts[0].parse().unwrap_or(0),
                parts[1].parse().unwrap_or(0),
                parts[2].parse().unwrap_or(0),
                parts[3].parse().unwrap_or(0),
            ));
        }
    }
    
    // 提取 corner='...' 和 fade='...' (存储到 custom)
    
    ImagePath { path, dest, corner, fade }
}
```

## 五、实施时间表

### 第 1 周：基础兼容
- [ ] 阶段 1: 根元素和基础容器
- [ ] 阶段 2: UI 元素映射
- [ ] 阶段 3.1-3.2: 基础属性映射和内边距转换

### 第 2 周：高级属性
- [ ] 阶段 3.3-3.8: 对齐、图片、字体、边框、浮动定位等属性
- [ ] 阶段 4: 字体系统

### 第 3 周：渲染器扩展
- [ ] 阶段 6: 渲染器扩展
- [ ] 阶段 5: 窗口定义（可选）

### 第 4 周：测试和优化
- [ ] 阶段 7: 测试和验证
- [ ] 性能优化
- [ ] 文档更新

## 六、风险评估

1. **兼容性风险**: 低 - 可以同时支持两种格式
2. **性能风险**: 低 - 属性映射在解析时完成，不影响运行时性能
3. **维护风险**: 低 - 代码结构清晰，易于维护
4. **测试风险**: 中 - 需要充分测试所有 NSIS 布局文件

## 七、总结

采用**完全兼容方案**，分阶段实施，确保：
1. ✅ 可以直接使用 NSIS 的布局文件
2. ✅ 保持向后兼容，现有布局文件仍然可用
3. ✅ 与 NSIS 完全一致，便于参考和调试
4. ✅ 代码结构清晰，易于维护和扩展

