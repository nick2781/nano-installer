# NSIS 布局格式兼容性迁移计划

## 目标
让我们的布局系统完全兼容 NSIS 的布局格式，可以直接使用 NSIS 的 XML 布局文件。

## 当前差异分析

### 1. 根元素差异
- **NSIS**: `<Windows>` 作为根元素
- **当前**: `<Layout name="..." version="..." width="..." height="...">` 作为根元素

### 2. 布局容器差异
- **NSIS**: 
  - `<VerticalLayout>` - 垂直布局
  - `<HorizontalLayout>` - 水平布局
- **当前**: 
  - `<VBox>` - 垂直布局
  - `<HBox>` - 水平布局

### 3. 元素命名差异
- **NSIS**: `<CheckBox>`, `<Control>`, `<Container>`, `<RichEdit>`, `<Slider>`
- **当前**: `<Checkbox>`, `<Spacer>`, 无对应, `<TextInput>`, `<ProgressBar>`

### 4. 属性命名差异

| NSIS 属性 | 当前属性 | 说明 | 优先级 |
|-----------|----------|------|--------|
| `name` | `id` | 元素标识符 | **高** - 必须支持 |
| `bkcolor` | `background` (颜色) | 背景颜色 | **高** - 必须支持 |
| `bkimage` | `background` (图片) | 背景图片 | **高** - 必须支持 |
| `textcolor` | `color` | 文本颜色 | **高** - 必须支持 |
| `inset` | `padding` | 内边距（格式不同） | **高** - 必须支持 |
| `valign` | 无 | 垂直对齐 | **中** - 需要添加 |
| `align` | `align` | 水平对齐 | **中** - 语义可能不同 |
| `textpadding` | `textpadding` | 文本内边距 | **高** - 已支持 |
| `borderround` | 无 | 圆角 | **中** - 需要添加 |
| `font` | `font_size` | 字体（NSIS 用 ID，我们用大小） | **高** - 需要映射 |
| `float` | `position` | 绝对定位 | **中** - 需要支持 |
| `pos` | `position` | 位置（格式不同） | **中** - 需要支持 |
| `normalimage` | `normalimage` | 正常状态图片 | **高** - 已支持 |
| `hotimage` | `hotimage` | 悬停状态图片 | **高** - 已支持 |
| `pushedimage` | `pushedimage` | 按下状态图片 | **高** - 已支持 |
| `selected` | `selected` | 选中状态 | **高** - 已支持 |
| `visible` | `visible` | 可见性 | **高** - 已支持 |
| `cursor` | 无 | 鼠标样式 | **低** - 可选 |
| `margin` | 无 | 外边距 | **低** - 可选 |
| `readonly` | 无 | 只读（RichEdit） | **中** - 需要支持 |
| `autohscroll` | 无 | 自动水平滚动 | **低** - 可选 |
| `wantreturn` | 无 | 接受回车 | **低** - 可选 |
| `multiline` | 无 | 多行 | **低** - 可选 |
| `textalign` | `align` | 文本对齐 | **中** - 需要区分 |

### 5. 属性格式差异

#### `inset` vs `padding`
- **NSIS**: `inset="left,top,right,bottom"` (例如: `inset="80,70,80,70"`)
- **当前**: `padding="top,right,bottom,left"` (例如: `padding="70,80,70,80"`)

#### `pos` vs `position`
- **NSIS**: `pos="x1,y1,x2,y2"` (矩形区域，例如: `pos="0,716,1148,1036"`)
- **当前**: `position="x,y"` (点坐标，例如: `position="1052,32"`)

## 迁移方案

### 方案 A: 完全兼容 NSIS 格式（推荐）
**优点**: 
- 可以直接使用 NSIS 的布局文件
- 无需维护两套布局文件
- 与 NSIS 完全一致，便于参考和调试

**缺点**: 
- 需要修改解析器和渲染器
- 需要处理属性映射和格式转换

**实施步骤**:
1. **扩展 XML 解析器** (`installer/lib/src/layout/xml_parser.rs`)
   - 支持 `<Windows>` 作为根元素（可选，兼容现有格式）
   - 支持 `<VerticalLayout>` 和 `<HorizontalLayout>`（同时保留 `<VBox>` 和 `<HBox>`）
   - 支持 `<CheckBox>`（同时保留 `<Checkbox>`）
   - 支持 `<Control>` 作为空白占位符（映射到 `<Spacer>`）
   - 支持 `<Container>` 作为容器（映射到 `<Spacer>` 或空容器）
   - 支持 `<RichEdit>`（映射到 `<TextInput>`）
   - 支持 `<Slider>`（映射到 `<ProgressBar>`）

2. **扩展属性解析** (`installer/lib/src/layout/xml_parser.rs`)
   - 支持 `name` 属性（映射到 `id`）
   - 支持 `bkcolor` 属性（映射到 `background` 颜色）
   - 支持 `bkimage` 属性（映射到 `background` 图片）
   - 支持 `textcolor` 属性（映射到 `color`）
   - 支持 `inset` 属性（转换为 `padding` 格式）
   - 支持 `valign` 属性（添加到 `custom` 或新字段）
   - 支持 `borderround` 属性（添加到 `custom`）
   - 支持 `font` 属性（需要字体 ID 到字体大小的映射表）
   - 支持 `float` 和 `pos` 属性（转换为 `position`）
   - 支持其他 NSIS 特有属性（添加到 `custom`）

3. **扩展渲染器** (`installer/lib/src/ui/layout_renderer.rs`)
   - 处理 `valign` 属性（垂直对齐）
   - 处理 `borderround` 属性（圆角）
   - 处理 `font` ID 映射（需要字体配置）
   - 处理 `pos` 格式（矩形区域定位）
   - 处理 `readonly` 属性（TextInput）

4. **字体系统**
   - 创建字体 ID 到字体大小的映射表
   - 支持从配置文件或 XML 中定义字体（参考 NSIS 的 `<Font>` 元素）

5. **测试和验证**
   - 直接使用 NSIS 的 `configpage2x.xml` 作为测试
   - 验证所有元素和属性都能正确解析和渲染

### 方案 B: 保持当前格式，添加转换工具
**优点**: 
- 不破坏现有代码
- 可以逐步迁移

**缺点**: 
- 需要维护两套格式
- 需要额外的转换工具

## 推荐实施方案

**采用方案 A（完全兼容）**，分阶段实施：

### 阶段 1: 基础兼容（1-2 天）
- [ ] 支持 `<Windows>` 根元素（可选）
- [ ] 支持 `<VerticalLayout>` 和 `<HorizontalLayout>`
- [ ] 支持 `<CheckBox>`、`<Control>`、`<Container>`
- [ ] 支持 `name`、`bkcolor`、`bkimage`、`textcolor` 属性
- [ ] 支持 `inset` 属性（格式转换）

### 阶段 2: 高级属性（1-2 天）
- [ ] 支持 `valign`、`borderround` 属性
- [ ] 支持 `font` ID 映射
- [ ] 支持 `float` 和 `pos` 属性
- [ ] 支持 `<RichEdit>` 和 `<Slider>`

### 阶段 3: 字体系统（1 天）
- [ ] 实现字体 ID 到字体大小的映射
- [ ] 支持从 XML 中定义字体（`<Font>` 元素）

### 阶段 4: 测试和优化（1 天）
- [ ] 直接使用 NSIS 布局文件测试
- [ ] 修复兼容性问题
- [ ] 性能优化

## 实施细节

### 1. 属性映射表
```rust
// 在 xml_parser.rs 中
fn map_nsis_attribute(key: &str, value: &str) -> (String, String) {
    match key {
        "name" => ("id".to_string(), value.to_string()),
        "bkcolor" => ("background".to_string(), value.to_string()),
        "bkimage" => ("background".to_string(), value.to_string()),
        "textcolor" => ("color".to_string(), value.to_string()),
        "inset" => {
            // 转换格式: "left,top,right,bottom" -> "top,right,bottom,left"
            let parts: Vec<&str> = value.split(',').collect();
            if parts.len() == 4 {
                let padding = format!("{},{},{},{}", parts[1], parts[2], parts[3], parts[0]);
                ("padding".to_string(), padding)
            } else {
                (key.to_string(), value.to_string())
            }
        },
        // ... 其他映射
        _ => (key.to_string(), value.to_string()),
    }
}
```

### 2. 元素类型映射
```rust
fn map_nsis_element_type(name: &str) -> ElementType {
    match name {
        "VerticalLayout" => ElementType::VBox,
        "HorizontalLayout" => ElementType::HBox,
        "CheckBox" => ElementType::Checkbox,
        "Control" => ElementType::Spacer,
        "Container" => ElementType::Spacer, // 或新类型
        "RichEdit" => ElementType::TextInput,
        "Slider" => ElementType::ProgressBar,
        _ => parse_element_type(name)?, // 回退到现有解析
    }
}
```

### 3. 字体 ID 映射
```rust
// 从 NSIS XML 中解析 <Font> 元素
struct FontConfig {
    id: u32,
    name: String,
    size: f32,
    bold: bool,
}

// 在解析时建立映射表
let font_map: HashMap<u32, FontConfig> = ...;
```

## 风险评估

1. **兼容性风险**: 低 - 可以同时支持两种格式
2. **性能风险**: 低 - 属性映射在解析时完成，不影响运行时性能
3. **维护风险**: 低 - 代码结构清晰，易于维护

## 总结

建议采用**方案 A（完全兼容）**，分阶段实施。这样可以：
1. 直接使用 NSIS 的布局文件，无需转换
2. 与 NSIS 保持完全一致，便于参考和调试
3. 保持向后兼容，现有的布局文件仍然可以工作

