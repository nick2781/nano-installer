# 实施策略：在当前代码基础上修改

## 一、基本原则

1. **不创建新项目**：在当前代码基础上修改
2. **备份策略**：重要修改前备份代码（注释或 `.bak` 文件）
3. **逐步实施**：分阶段修改，每个阶段确保代码可编译运行
4. **保留可用逻辑**：现有代码中可用的部分保留并复用

## 二、备份策略

### 方法 1: 注释备份
```rust
// ===== 旧代码备份 =====
// impl XmlParser {
//     pub fn parse_old_format(&self) { ... }
// }
// ===== 备份结束 =====

// 新代码
impl XmlParser {
    pub fn parse(&mut self) { ... }
}
```

### 方法 2: 文件备份
- 修改前：`cp xml_parser.rs xml_parser.rs.bak`
- 修改后：保留 `.bak` 文件作为参考

### 方法 3: Git 提交
- 每个阶段修改前提交一次
- 方便回滚和对比

## 三、修改顺序

### 阶段 1: 解析器扩展（最小改动）
1. 在 `XmlParser` 中添加 `font_map` 字段
2. 添加字体解析方法
3. 扩展 `parse_element_type` 方法，支持新元素类型
4. 扩展 `parse_attributes` 方法，支持新属性
5. 测试：确保现有布局文件仍能解析

### 阶段 2: 数据结构扩展（最小改动）
1. 在 `ElementAttributes` 中添加新字段（使用 `Option`，保持向后兼容）
2. 在 `ElementType` 中添加新变体
3. 测试：确保现有代码仍能编译

### 阶段 3: 渲染器扩展（逐步添加）
1. 添加新元素的渲染方法
2. 修改现有渲染方法，支持新属性
3. 测试：每个方法单独测试

### 阶段 4: 集成测试
1. 使用 NSIS 布局文件测试
2. 修复问题
3. 清理备份代码（可选）

## 四、当前支持的标签（基于已读取的文件）

### 必须支持的标签（14个）
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

### 优先级
- **P0（必须）**: `<Windows>`, `<VerticalLayout>`, `<HorizontalLayout>`, `<Button>`, `<CheckBox>`, `<Label>`, `<Control>`, `<Container>`
- **P1（重要）**: `<Window>`, `<Font>`, `<RichEdit>`, `<Slider>`
- **P2（可选）**: `<TabLayout>`, `<Include>`

## 五、属性支持优先级

### P0（必须支持）
- `name`, `width`, `height`, `visible`, `enabled`
- `bkcolor`, `bkimage`, `textcolor`
- `inset`, `padding`, `textpadding`
- `align`, `valign`
- `normalimage`, `hotimage`, `pushedimage`, `disabledimage`
- `font` (字体 ID)
- `text`, `selected`

### P1（重要）
- `margin`, `borderround`, `bordercolor`, `bordersize`
- `float`, `pos`
- `textalign` (Label)
- `normalhotimage`, `selectedimage`, `selectedhotimage` (CheckBox)
- `readonly`, `multiline` (RichEdit)
- `min`, `max`, `value`, `foreimage` (Slider)

### P2（可选）
- `focusedimage`, `showhtml`, `cursor`
- `autohscroll`, `wantreturn`, `wantctrlreturn` (RichEdit)
- `thumbsize`, `mouse` (Slider)
- Window 特有属性（shadow 等）

## 六、实施检查清单

### 解析器 (`xml_parser.rs`)
- [ ] 备份现有代码
- [ ] 添加 `font_map: HashMap<u32, FontConfig>` 字段
- [ ] 实现 `parse_font()` 方法
- [ ] 扩展 `parse_element_type()` 支持新元素类型
- [ ] 扩展 `parse_attributes()` 支持新属性
- [ ] 实现 `parse_image_path()` 方法（支持 `file`, `dest`, `corner`, `fade`）
- [ ] 实现 `parse_inset()` 方法（格式：`left,top,right,bottom`）
- [ ] 实现 `parse_pos()` 方法（格式：`x1,y1,x2,y2`）
- [ ] 测试：现有布局文件仍能解析

### 数据结构 (`element.rs`)
- [ ] 备份现有代码
- [ ] 在 `ElementAttributes` 中添加新字段（使用 `Option`）
- [ ] 在 `ElementType` 中添加新变体
- [ ] 测试：代码能编译

### 渲染器 (`layout_renderer.rs`)
- [ ] 备份现有代码
- [ ] 添加 `render_vertical_layout()` 方法
- [ ] 添加 `render_horizontal_layout()` 方法
- [ ] 添加 `render_container()` 方法
- [ ] 添加 `render_control()` 方法
- [ ] 修改 `render_button()` 支持新属性
- [ ] 修改 `render_checkbox()` 支持新属性
- [ ] 修改 `render_label()` 支持新属性
- [ ] 添加 `render_richedit()` 方法
- [ ] 添加 `render_slider()` 方法
- [ ] 测试：每个方法单独测试

### 集成测试
- [ ] 使用 `configpage2x.xml` 测试
- [ ] 使用 `msgbox2x.xml` 测试
- [ ] 使用其他页面布局文件测试
- [ ] 修复所有问题

## 七、注意事项

1. **保持向后兼容**：新字段使用 `Option`，不影响现有代码
2. **逐步测试**：每个阶段修改后立即测试
3. **保留备份**：重要修改前备份，确认无误后再删除
4. **文档更新**：修改后更新相关文档

