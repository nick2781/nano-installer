//! XML布局解析器
//! 
//! 解析简化的XML布局格式并生成内部布局树

use crate::layout::{LayoutTree, LayoutElement, ElementType, ElementAttributes};
use crate::layout::layout_tree::LayoutMetadata;
use crate::layout::dimension::{Dimension, Edges};
use crate::layout::style_props::*;
use roxmltree::Document;
use std::collections::HashMap;

/// 字体配置
#[derive(Debug, Clone)]
pub struct FontConfig {
    pub id: u32,
    pub name: String,
    pub size: f32,
    pub bold: bool,
    pub default: bool,
}

impl FontConfig {
    /// 从全局 NSIS 默认字体表创建 FontConfig（用于当前布局文件未显式声明 <Font> 的情况）
    /// 这些值来源于示例中的 `install.xml`，用于还原 NSIS 下的视觉效果。
    pub fn from_default_table(id: u32) -> Option<Self> {
        let (name, size, bold) = match id {
            0 => ("微软雅黑", 12.0, false),
            1 => ("微软雅黑", 24.0, false),
            2 => ("微软雅黑", 14.0, false),
            3 => ("微软雅黑", 14.0, true),
            4 => ("微软雅黑", 28.0, false),
            5 => ("微软雅黑", 28.0, true),
            6 => ("Microsoft YaHei", 14.0, true),
            7 => ("Microsoft YaHei", 28.0, true),
            _ => return None,
        };

        Some(FontConfig {
            id,
            name: name.to_string(),
            size,
            bold,
            default: id == 0,
        })
    }
}

/// 图片路径（支持 file, dest, corner, fade）
#[derive(Debug, Clone, PartialEq)]
pub struct ImagePath {
    pub file: String,
    pub dest: Option<(u32, u32, u32, u32)>,  // x1, y1, x2, y2
    pub corner: Option<(u32, u32, u32, u32)>, // x1, y1, x2, y2
    pub fade: Option<u8>,  // 0-255
}

/// XML解析器
pub struct XmlParser {
    /// 解析选项
    options: ParserOptions,
    /// 字体映射表（字体 ID -> 字体配置）
    font_map: HashMap<u32, FontConfig>,
}

/// 解析选项
#[derive(Debug, Clone)]
pub struct ParserOptions {
    /// 是否严格验证
    pub strict_validation: bool,
    /// 是否自动修复常见错误
    pub auto_fix: bool,
    /// 是否收集统计信息
    pub collect_stats: bool,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            strict_validation: true,
            auto_fix: false,
            collect_stats: true,
        }
    }
}

/// 解析错误
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("XML解析错误: {0}")]
    XmlError(String),
    
    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("无效的元素类型: {0}")]
    InvalidElementType(String),
    
    #[error("无效的属性值: {0}")]
    InvalidAttributeValue(String),
    
    #[error("缺少必需属性: {0}")]
    MissingRequiredAttribute(String),
    
    #[error("验证失败: {0:?}")]
    ValidationFailed(Vec<String>),
    
    #[error("不支持的嵌套: {0} 不能包含 {1}")]
    UnsupportedNesting(String, String),
}

impl XmlParser {
    /// 创建新的解析器
    pub fn new() -> Self {
        Self {
            options: ParserOptions::default(),
            font_map: HashMap::new(),
        }
    }

    /// 创建带选项的解析器
    pub fn with_options(options: ParserOptions) -> Self {
        Self { 
            options,
            font_map: HashMap::new(),
        }
    }
    
    /// 根据 DPI 选择合适的布局文件
    pub fn select_layout_file(base_path: &str, dpi: f32) -> String {
        let dpi_threshold = 144.0; // 2x DPI 阈值
        
        if dpi >= dpi_threshold {
            // 尝试 @2x 版本
            if let Some(dot_pos) = base_path.rfind('.') {
                let (name, ext) = base_path.split_at(dot_pos);
                format!("{}@2x{}", name, ext)
            } else {
                format!("{}@2x", base_path)
            }
        } else {
            base_path.to_string()
        }
    }
    
    /// 检测系统 DPI
    pub fn detect_system_dpi() -> f32 {
        #[cfg(target_os = "windows")] {
            // 暂时返回默认 DPI，稍后实现真正的 DPI 检测
            96.0
        }
        #[cfg(not(target_os = "windows"))] {
            96.0 // 默认 DPI
        }
    }

    /// 解析XML文件
    pub fn parse_file<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<LayoutTree, ParseError> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy();
        
        // 检测系统 DPI 并选择合适的布局文件
        let dpi = Self::detect_system_dpi();
        let selected_path = Self::select_layout_file(&path_str, dpi);
        
        // 如果 @2x 文件不存在，回退到原始文件
        let final_path = if std::path::Path::new(&selected_path).exists() {
            selected_path
        } else {
            path_str.to_string()
        };
        
        let content = std::fs::read_to_string(&final_path)?;
        self.parse_string(&content)
    }

    /// 解析XML字符串
    pub fn parse_string(&mut self, xml: &str) -> Result<LayoutTree, ParseError> {
        // 使用 roxmltree 解析 XML
        let doc = Document::parse(xml).map_err(|e| {
            // 输出详细的错误信息用于调试
            let error_msg = format!("XML解析失败: {}", e);
            tracing::error!("roxmltree 解析错误: {}", error_msg);
            #[cfg(debug_assertions)]
            {
                eprintln!("[DEBUG] roxmltree 错误详情: {:?}", e);
                let pos = e.pos();
                eprintln!("[DEBUG] 错误位置: line {}, column {}", pos.row, pos.col);
                // 尝试找到错误位置的字节偏移
                let mut byte_pos: usize = 0;
                for (i, line) in xml.lines().enumerate() {
                    if i + 1 == pos.row as usize {
                        byte_pos += ((pos.col as usize).saturating_sub(1)).min(line.len());
                        break;
                    }
                    byte_pos += line.len() + 1; // +1 for newline
                }
                if byte_pos < xml.len() {
                    let start = byte_pos.saturating_sub(20);
                    let end = (byte_pos + 20).min(xml.len());
                    eprintln!("[DEBUG] 错误位置周围的文本: {:?}", &xml[start..end]);
                }
            }
            ParseError::XmlError(error_msg)
        })?;
        
        let root = doc.root_element();
        let root_name = root.tag_name().name();
        
        // 清空字体映射表（每次解析新文件时重置）
        self.font_map.clear();
        
        // 新格式: <Page> 作为根元素
        if root_name == "Page" {
            return self.parse_new_format(&root);
        }

        // 旧格式: 只支持 <Windows> 和 <Window> 作为根元素
        if !matches!(root_name, "Windows" | "Window") {
            return Err(ParseError::XmlError(format!(
                "不支持的根元素: {}，支持 <Page> (新格式) 或 <Windows>/<Window> (旧格式)",
                root_name
            )));
        }
        
        let mut metadata = LayoutMetadata::default();
        
        // 如果是 <Window>，解析窗口属性
        if root_name == "Window" {
            self.parse_window_metadata(&root, &mut metadata)?;
        }
        
        // 解析 <Font> 元素（在 <Window> 或 <Windows> 下）
        for child in root.children() {
            if child.is_element() && child.tag_name().name() == "Font" {
                let font_config = self.parse_font(&child)?;
                self.font_map.insert(font_config.id, font_config);
            }
        }
        
        // 创建一个虚拟的 Page 元素来包装布局
        // 查找第一个布局容器（VerticalLayout 或 HorizontalLayout）
        // 或者查找 TabLayout（包含 Include）
        let layout_node = root.children()
            .find(|n| {
                n.is_element() && matches!(
                    n.tag_name().name(),
                    "VerticalLayout" | "HorizontalLayout" | "TabLayout"
                )
            })
            .ok_or_else(|| ParseError::ValidationFailed(vec!["缺少布局容器元素".to_string()]))?;
        
        // 递归查找第一个有 width 和 height 的布局容器（用于设置 Page 元素的尺寸）
        // 如果第一个布局容器没有 width/height，查找其子元素中第一个有 width/height 的
        let mut page_attributes = ElementAttributes::new();
        
        // 先检查第一个布局容器是否有 width/height
        let mut has_size = false;
        for attr in layout_node.attributes() {
            match attr.name() {
                "width" => {
                    if let Ok(w) = attr.value().parse::<f32>() {
                        page_attributes.width = Some(w);
                        has_size = true;
                    }
                }
                "height" => {
                    if let Ok(h) = attr.value().parse::<f32>() {
                        page_attributes.height = Some(h);
                        has_size = true;
                    }
                }
                _ => {}
            }
        }
        
        // 如果第一个布局容器没有 width/height，递归查找子元素中第一个有 width/height 的
        if !has_size {
            fn find_layout_with_size_attrs(node: &roxmltree::Node) -> Option<(f32, f32)> {
                for child in node.children() {
                    if child.is_element() && matches!(
                        child.tag_name().name(),
                        "VerticalLayout" | "HorizontalLayout"
                    ) {
                        // 检查这个子元素是否有 width 和 height
                        let mut width_opt = None;
                        let mut height_opt = None;
                        for attr in child.attributes() {
                            match attr.name() {
                                "width" => {
                                    if let Ok(w) = attr.value().parse::<f32>() {
                                        width_opt = Some(w);
                                    }
                                }
                                "height" => {
                                    if let Ok(h) = attr.value().parse::<f32>() {
                                        height_opt = Some(h);
                                    }
                                }
                                _ => {}
                            }
                        }
                        if let (Some(w), Some(h)) = (width_opt, height_opt) {
                            return Some((w, h));
                        }
                        // 递归查找
                        if let Some((w, h)) = find_layout_with_size_attrs(&child) {
                            return Some((w, h));
                        }
                    }
                }
                None
            }
            
            if let Some((w, h)) = find_layout_with_size_attrs(&layout_node) {
                page_attributes.width = Some(w);
                page_attributes.height = Some(h);
            }
        }
        
        // 解析布局容器元素
        let layout_element = self.parse_element(&layout_node)?;
        
        // 创建一个 Page 元素来包装布局容器（用于验证和兼容性）
        // 将布局容器的 width 和 height 设置到 Page 元素上
        let page_element = LayoutElement::with_attributes(
            ElementType::Page,
            page_attributes,
        ).add_child(layout_element);
        
        let tree = LayoutTree::with_metadata(page_element, metadata);

        // 验证布局树
        if let Err(validation_errors) = tree.validate() {
            if self.options.strict_validation {
                return Err(ParseError::ValidationFailed(validation_errors));
            }
        }

        Ok(tree)
    }

    /// 解析元素节点
    fn parse_element(&self, node: &roxmltree::Node) -> Result<LayoutElement, ParseError> {
        let element_type = self.parse_element_type(node.tag_name().name())?;
        let attributes = self.parse_attributes(node)?;
        
        // 调试日志：检查 moreconfiginfo2x 的解析
        if let Some(id) = &attributes.id {
            if id == "moreconfiginfo2x" {
                let has_float = attributes.get_custom("float").is_some();
                let has_pos = attributes.get_custom("pos").is_some();
                let is_absolute = attributes.get_custom("is_absolute").map(|s| s == "true").unwrap_or(false);
                eprintln!("[解析] moreconfiginfo2x 解析: float={}, pos={}, is_absolute={}", has_float, has_pos, is_absolute);
                if has_pos {
                    if let Some(pos_str) = attributes.get_custom("pos") {
                        eprintln!("[解析] moreconfiginfo2x pos 值: {}", pos_str);
                    }
                }
                // 检查父元素
                if let Some(parent) = node.parent() {
                    if parent.is_element() {
                        eprintln!("[解析] moreconfiginfo2x 的父元素: {}", parent.tag_name().name());
                    }
                }
            }
        }
        
        let mut element = LayoutElement::with_attributes(element_type.clone(), attributes);
        
        // 递归解析子元素
        let mut child_count = 0;
        for child in node.children() {
            if child.is_element() {
                child_count += 1;
                let child_tag = child.tag_name().name();
                let child_element = self.parse_element(&child)?;
                // 调试日志：检查 moreconfiginfo2x 是否被解析为某个元素的子元素
                if let Some(child_id) = &child_element.attributes.id {
                    if child_id == "moreconfiginfo2x" {
                        let element_type_str = format!("{:?}", element_type);
                        let element_id = element.attributes.id.as_ref().map(|s| s.as_str()).unwrap_or("unnamed");
                        eprintln!("[解析] moreconfiginfo2x 被解析为 {} (id: {}, XML标签: {}) 的第 {} 个子元素", 
                            element_type_str, element_id, child_tag, child_count);
                    }
                }
                element.children.push(child_element);
            } else if child.is_text() {
                // 处理文本内容
                let text = child.text().unwrap_or("").trim();
                if !text.is_empty() && element.attributes.text.is_none() {
                    element.attributes.text = Some(text.to_string());
                }
            }
            // 忽略注释等其他节点类型
        }
        
        Ok(element)
    }

    // ───────────────────────────────────────────────────────
    // 新格式解析 (<Page> 根元素, CSS Flexbox 风格属性)
    // ───────────────────────────────────────────────────────

    /// 解析新格式 XML (<Page> 根元素)
    fn parse_new_format(&self, root: &roxmltree::Node) -> Result<LayoutTree, ParseError> {
        let element = self.parse_new_element(root)?;
        let tree = LayoutTree::new(element);

        if self.options.strict_validation {
            if let Err(errors) = tree.validate() {
                return Err(ParseError::ValidationFailed(errors));
            }
        }

        Ok(tree)
    }

    /// 解析新格式元素节点
    fn parse_new_element(&self, node: &roxmltree::Node) -> Result<LayoutElement, ParseError> {
        let tag = node.tag_name().name();
        let element_type = self.parse_new_element_type(tag)?;

        // 解析 CSS-style 属性
        let (flex_style, visual_style, widget_props) = self.parse_new_attributes(node, &element_type)?;

        // 同时生成旧格式 attributes (用于渲染器向后兼容, Phase 3 前需要)
        let legacy_attrs = self.new_to_legacy_attrs(&flex_style, &visual_style, &widget_props, &element_type);

        let mut element = LayoutElement::with_attributes(element_type, legacy_attrs);
        element.flex_style = Some(flex_style);
        element.visual_style = Some(visual_style);
        element.widget_props = Some(widget_props);

        // 递归解析子元素
        for child in node.children() {
            if child.is_element() {
                element.children.push(self.parse_new_element(&child)?);
            } else if child.is_text() {
                let text = child.text().unwrap_or("").trim();
                if !text.is_empty() {
                    // 文本内容作为 text (支持 @i18n)
                    if let Some(wp) = &mut element.widget_props {
                        if wp.text.is_none() {
                            wp.text = Some(text.to_string());
                        }
                    }
                    if element.attributes.text.is_none() {
                        element.attributes.text = Some(text.to_string());
                    }
                }
            }
        }

        Ok(element)
    }

    /// 新格式元素类型映射
    fn parse_new_element_type(&self, name: &str) -> Result<ElementType, ParseError> {
        match name {
            "Page" => Ok(ElementType::Page),
            "Box" | "VBox" => Ok(ElementType::VBox),
            "HBox" => Ok(ElementType::HBox),
            "Overlay" => Ok(ElementType::Overlay),
            "Button" => Ok(ElementType::Button),
            "Label" => Ok(ElementType::Label),
            "Checkbox" => Ok(ElementType::Checkbox),
            "TextInput" => Ok(ElementType::TextInput),
            "Image" => Ok(ElementType::Image),
            "ProgressBar" => Ok(ElementType::ProgressBar),
            "Spacer" => Ok(ElementType::Spacer),
            "Divider" => Ok(ElementType::Divider),
            "Select" | "Dropdown" => Ok(ElementType::Select),
            "Option" => Ok(ElementType::Label), // Option items rendered as Label in Select context
            _ => Err(ParseError::InvalidElementType(name.to_string())),
        }
    }

    /// 解析新格式属性 -> (FlexStyle, VisualStyle, WidgetProps)
    fn parse_new_attributes(
        &self,
        node: &roxmltree::Node,
        element_type: &ElementType,
    ) -> Result<(FlexStyle, VisualStyle, WidgetProps), ParseError> {
        let mut flex = FlexStyle::default();
        let mut visual = VisualStyle::default();
        let mut widget = WidgetProps::default();

        // VBox/Page 默认 flex-direction: column
        match element_type {
            ElementType::VBox | ElementType::Page => {
                flex.flex_direction = FlexDirection::Column;
            }
            _ => {}
        }

        for attr in node.attributes() {
            let key = attr.name();
            let val = attr.value();

            match key {
                // ── 布局属性 ──
                "width" => flex.width = Dimension::parse(val).unwrap_or(Dimension::Auto),
                "height" => flex.height = Dimension::parse(val).unwrap_or(Dimension::Auto),
                "min-width" => flex.min_width = Dimension::parse(val).unwrap_or(Dimension::Auto),
                "min-height" => flex.min_height = Dimension::parse(val).unwrap_or(Dimension::Auto),
                "max-width" => flex.max_width = Dimension::parse(val).unwrap_or(Dimension::Auto),
                "max-height" => flex.max_height = Dimension::parse(val).unwrap_or(Dimension::Auto),

                "flex-direction" => {
                    flex.flex_direction = FlexDirection::parse(val).unwrap_or(flex.flex_direction);
                }
                "flex-wrap" => {
                    flex.flex_wrap = FlexWrap::parse(val).unwrap_or(FlexWrap::NoWrap);
                }
                "justify-content" => {
                    flex.justify_content = JustifyContent::parse(val).unwrap_or(JustifyContent::FlexStart);
                }
                "align-items" => {
                    flex.align_items = AlignItems::parse(val).unwrap_or(AlignItems::Stretch);
                }
                "align-content" => {
                    flex.align_content = AlignContent::parse(val).unwrap_or(AlignContent::Stretch);
                }
                "gap" => {
                    flex.gap = val.parse::<f32>().unwrap_or(0.0);
                }
                "flex-grow" => {
                    flex.flex_grow = val.parse::<f32>().unwrap_or(0.0);
                }
                "flex-shrink" => {
                    flex.flex_shrink = val.parse::<f32>().unwrap_or(1.0);
                }
                "flex-basis" => {
                    flex.flex_basis = Dimension::parse(val).unwrap_or(Dimension::Auto);
                }
                "align-self" => {
                    flex.align_self = AlignSelf::parse(val).unwrap_or(AlignSelf::Auto);
                }

                // margin 简写 + 单边
                "margin" => {
                    if let Some(edges) = Edges::parse_shorthand(val) {
                        flex.margin = edges;
                    }
                }
                "margin-top" => {
                    flex.margin.top = Dimension::parse(val).unwrap_or(Dimension::Px(0.0));
                }
                "margin-right" => {
                    flex.margin.right = Dimension::parse(val).unwrap_or(Dimension::Px(0.0));
                }
                "margin-bottom" => {
                    flex.margin.bottom = Dimension::parse(val).unwrap_or(Dimension::Px(0.0));
                }
                "margin-left" => {
                    flex.margin.left = Dimension::parse(val).unwrap_or(Dimension::Px(0.0));
                }

                // padding 简写 + 单边
                "padding" => {
                    if let Some(edges) = Edges::parse_shorthand(val) {
                        flex.padding = edges;
                    }
                }
                "padding-top" => {
                    flex.padding.top = Dimension::parse(val).unwrap_or(Dimension::Px(0.0));
                }
                "padding-right" => {
                    flex.padding.right = Dimension::parse(val).unwrap_or(Dimension::Px(0.0));
                }
                "padding-bottom" => {
                    flex.padding.bottom = Dimension::parse(val).unwrap_or(Dimension::Px(0.0));
                }
                "padding-left" => {
                    flex.padding.left = Dimension::parse(val).unwrap_or(Dimension::Px(0.0));
                }

                // 定位
                "position" => {
                    flex.position = Position::parse(val).unwrap_or(Position::Relative);
                }
                "top" => flex.top = Dimension::parse(val).unwrap_or(Dimension::Auto),
                "right" => flex.right = Dimension::parse(val).unwrap_or(Dimension::Auto),
                "bottom" => flex.bottom = Dimension::parse(val).unwrap_or(Dimension::Auto),
                "left" => flex.left = Dimension::parse(val).unwrap_or(Dimension::Auto),

                // spacing (alias for gap)
                "spacing" => {
                    flex.gap = val.parse::<f32>().unwrap_or(0.0);
                }

                // ── 视觉属性 ──
                "background" => visual.background = Some(val.to_string()),
                "background-image" => visual.background_image = Some(val.to_string()),
                "color" => visual.color = Some(val.to_string()),
                "font-size" => visual.font_size = val.parse::<f32>().ok(),
                "font-weight" => visual.font_weight = FontWeight::parse(val),
                "font-family" => visual.font_family = Some(val.to_string()),
                "text-align" => visual.text_align = TextAlign::parse(val),
                "border-radius" => visual.border_radius = val.parse::<f32>().ok(),
                "border-color" => visual.border_color = Some(val.to_string()),
                "border-width" => visual.border_width = val.parse::<f32>().ok(),
                "opacity" => visual.opacity = val.parse::<f32>().ok(),
                "visible" => {
                    visual.visible = self.parse_bool(val).unwrap_or(true);
                }
                "enabled" => {
                    visual.enabled = self.parse_bool(val).unwrap_or(true);
                }

                // ── 控件属性 ──
                "id" => widget.id = Some(val.to_string()),
                "text" => widget.text = Some(val.to_string()),

                // Button images
                "normal-image" => widget.normal_image = Some(val.to_string()),
                "hover-image" => widget.hover_image = Some(val.to_string()),
                "pressed-image" => widget.pressed_image = Some(val.to_string()),
                "disabled-image" => widget.disabled_image = Some(val.to_string()),

                // Checkbox images
                "checked-image" => widget.checked_image = Some(val.to_string()),
                "unchecked-image" => widget.unchecked_image = Some(val.to_string()),
                "checked" => widget.checked = self.parse_bool(val).ok(),

                // Action (button behavior declaration)
                "action" => widget.action = Some(val.to_string()),

                // ProgressBar
                "progress" => widget.progress = val.parse::<f32>().ok(),
                "bar-image" => widget.bar_image = Some(val.to_string()),
                "track-image" => widget.track_image = Some(val.to_string()),

                // Image
                "src" => widget.src = Some(val.to_string()),

                // TextInput
                "placeholder" => widget.placeholder = Some(val.to_string()),
                "readonly" => widget.readonly = self.parse_bool(val).ok(),
                "multiline" => widget.multiline = self.parse_bool(val).ok(),

                // 通用
                "max-lines" => widget.max_lines = val.parse::<usize>().ok(),
                "wrap" => widget.wrap = self.parse_bool(val).ok(),

                // 兼容旧格式属性名 (无 hyphen)
                "normalimage" | "hotimage" | "pushedimage" | "disabledimage" |
                "selectedimage" | "foreimage" | "normalhotimage" | "selectedhotimage" |
                "focusedimage" => {
                    widget.custom.insert(key.to_string(), val.to_string());
                }

                // 其他自定义属性
                _ => {
                    widget.custom.insert(key.to_string(), val.to_string());
                }
            }
        }

        Ok((flex, visual, widget))
    }

    /// 将新格式属性转换为旧格式 ElementAttributes (向后兼容渲染器)
    fn new_to_legacy_attrs(
        &self,
        flex: &FlexStyle,
        visual: &VisualStyle,
        widget: &WidgetProps,
        _element_type: &ElementType,
    ) -> ElementAttributes {
        let mut attrs = ElementAttributes::new();

        attrs.id = widget.id.clone();
        attrs.text = widget.text.clone();

        // 尺寸 (只转换 Px 值)
        if let Dimension::Px(w) = flex.width { attrs.width = Some(w); }
        if let Dimension::Px(h) = flex.height { attrs.height = Some(h); }
        if let Dimension::Px(w) = flex.min_width { attrs.min_width = Some(w); }
        if let Dimension::Px(w) = flex.max_width { attrs.max_width = Some(w); }

        // padding
        let p = &flex.padding;
        let to_px = |d: Dimension| -> f32 { if let Dimension::Px(v) = d { v } else { 0.0 } };
        let pt = to_px(p.top);
        let pr = to_px(p.right);
        let pb = to_px(p.bottom);
        let pl = to_px(p.left);
        if pt != 0.0 || pr != 0.0 || pb != 0.0 || pl != 0.0 {
            attrs.padding = Some((pt, pr, pb, pl));
        }

        attrs.spacing = if flex.gap != 0.0 { Some(flex.gap) } else { None };
        attrs.flex = if flex.flex_grow != 0.0 { Some(flex.flex_grow) } else { None };

        attrs.visible = Some(visual.visible);
        attrs.enabled = Some(visual.enabled);

        // 背景
        if let Some(bg) = &visual.background {
            attrs.background = Some(bg.clone());
        }
        if let Some(bg_img) = &visual.background_image {
            attrs.background = Some(bg_img.clone());
        }

        attrs.color = visual.color.clone();

        // Icon/src for Image
        if let Some(src) = &widget.src {
            attrs.icon = Some(src.clone());
        }

        // Checkbox selected
        attrs.selected = widget.checked;

        // Progress
        attrs.progress = widget.progress;

        // 绝对定位
        if flex.position == Position::Absolute {
            attrs.custom.insert("is_absolute".to_string(), "true".to_string());
            if let Dimension::Px(x) = flex.left {
                if let Dimension::Px(y) = flex.top {
                    attrs.position = Some((x, y));
                }
            }
        }

        // Action -> custom (for backward compat with layout_renderer)
        if let Some(action) = &widget.action {
            attrs.custom.insert("action".to_string(), action.clone());
        }

        // Button images -> custom (旧格式)
        if let Some(img) = &widget.normal_image {
            attrs.custom.insert("normalimage".to_string(), img.clone());
        }
        if let Some(img) = &widget.hover_image {
            attrs.custom.insert("hotimage".to_string(), img.clone());
        }
        if let Some(img) = &widget.pressed_image {
            attrs.custom.insert("pushedimage".to_string(), img.clone());
        }
        if let Some(img) = &widget.disabled_image {
            attrs.custom.insert("disabledimage".to_string(), img.clone());
        }

        // Checkbox images -> custom
        if let Some(img) = &widget.unchecked_image {
            attrs.custom.insert("normalimage".to_string(), img.clone());
        }
        if let Some(img) = &widget.checked_image {
            attrs.custom.insert("selectedimage".to_string(), img.clone());
        }

        // Font size/weight -> custom
        if let Some(size) = visual.font_size {
            attrs.custom.insert("font_size".to_string(), size.to_string());
        }
        if let Some(FontWeight::Bold) = visual.font_weight {
            attrs.custom.insert("font_bold".to_string(), "true".to_string());
        }

        // border
        if let Some(r) = visual.border_radius {
            attrs.custom.insert("borderround".to_string(), format!("{},{}", r, r));
        }
        if let Some(c) = &visual.border_color {
            attrs.custom.insert("bordercolor".to_string(), c.clone());
        }
        if let Some(w) = visual.border_width {
            attrs.custom.insert("bordersize".to_string(), w.to_string());
        }

        // TextInput
        if let Some(readonly) = widget.readonly {
            attrs.custom.insert("readonly".to_string(), readonly.to_string());
        }
        if let Some(multiline) = widget.multiline {
            attrs.custom.insert("multiline".to_string(), multiline.to_string());
        }

        // text-align -> custom
        if let Some(align) = visual.text_align {
            let align_str = match align {
                TextAlign::Left => "left",
                TextAlign::Center => "center",
                TextAlign::Right => "right",
            };
            attrs.align = Some(align_str.to_string());
            attrs.custom.insert("textalign".to_string(), align_str.to_string());
        }

        // valign from align-items (简单映射)
        // 保留其他自定义属性
        for (k, v) in &widget.custom {
            attrs.custom.insert(k.clone(), v.clone());
        }

        attrs
    }

    // ───────────────────────────────────────────────────────
    // 旧格式解析 (NSIS <Windows>/<Window>)
    // ───────────────────────────────────────────────────────

    /// 解析元素类型 (旧格式)
    fn parse_element_type(&self, name: &str) -> Result<ElementType, ParseError> {
        match name {
            // NSIS 格式元素
            "Windows" => Ok(ElementType::Page),  // 根元素，映射为 Page
            "Window" => Ok(ElementType::Page),   // 窗口元素，映射为 Page
            "VerticalLayout" => Ok(ElementType::VBox),
            "HorizontalLayout" => Ok(ElementType::HBox),
            "Container" => Ok(ElementType::Spacer),
            "Control" => Ok(ElementType::Spacer),
            "Button" => Ok(ElementType::Button),
            "Label" => Ok(ElementType::Label),
            "CheckBox" => Ok(ElementType::Checkbox),
            "RichEdit" => Ok(ElementType::TextInput),
            "Slider" => Ok(ElementType::ProgressBar),
            "TabLayout" => Ok(ElementType::VBox),  // 暂时映射为 VBox
            "Include" => Ok(ElementType::Spacer),  // Include 暂时映射为空白占位符（后续可扩展为文件包含）
            "Font" => Err(ParseError::InvalidElementType("Font 需要特殊处理".to_string())),
            _ => Err(ParseError::InvalidElementType(name.to_string())),
        }
    }

    /// 解析属性
    fn parse_attributes(&self, node: &roxmltree::Node) -> Result<ElementAttributes, ParseError> {
        let mut attributes = ElementAttributes::new();

        for attr in node.attributes() {
            let key = attr.name();
            let value = attr.value();

            match key {
                // NSIS 格式属性
                "name" => {
                    // name 映射到 id
                    attributes.id = Some(value.to_string());
                }
                "text" => attributes.text = Some(value.to_string()),
                "text_i18n" => attributes.text_i18n = Some(value.to_string()),
                "width" => attributes.width = Some(self.parse_f32(value)?),
                "height" => attributes.height = Some(self.parse_f32(value)?),
                "min_width" => attributes.min_width = Some(self.parse_f32(value)?),
                "max_width" => attributes.max_width = Some(self.parse_f32(value)?),
                "padding" => {
                    // NSIS 格式：padding="left,top,right,bottom"，需要转换为 top,right,bottom,left
                    let padding = self.parse_inset(value)?;  // left,top,right,bottom
                    attributes.padding = Some((padding.1, padding.2, padding.3, padding.0));
                }
                "spacing" => attributes.spacing = Some(self.parse_f32(value)?),
                "bkcolor" => {
                    // bkcolor 映射到 background（颜色）
                    attributes.background = Some(value.to_string());
                }
                "bkimage" => {
                    // bkimage 映射到 background（图片）
                    attributes.background = Some(value.to_string());
                }
                "textcolor" => {
                    // textcolor 映射到 color
                    attributes.color = Some(value.to_string());
                }
                "align" => attributes.align = Some(value.to_string()),
                "icon" => attributes.icon = Some(value.to_string()),
                "visible" => attributes.visible = Some(self.parse_bool(value)?),
                "enabled" => attributes.enabled = Some(self.parse_bool(value)?),
                "selected" => attributes.selected = Some(self.parse_bool(value)?),
                "progress" => attributes.progress = Some(self.parse_f32(value)?),
                "position" => attributes.position = Some(self.parse_position(value)?),
                "wrap" => attributes.wrap = Some(self.parse_bool(value)?),
                "max_lines" => attributes.max_lines = value.parse().ok(),
                "flex" => attributes.flex = Some(self.parse_f32(value)?),
                "inset" => {
                    // inset="left,top,right,bottom" -> padding="top,right,bottom,left"
                    let inset = self.parse_inset(value)?;
                    attributes.padding = Some((inset.1, inset.2, inset.3, inset.0));
                }
                "margin" => {
                    // margin 存储到 custom
                    let margin = self.parse_inset(value)?;
                    attributes.custom.insert("margin".to_string(), format!("{},{},{},{}", margin.0, margin.1, margin.2, margin.3));
                }
                "textpadding" => {
                    // textpadding="left,top,right,bottom" 存储到 custom
                    let textpadding = self.parse_inset(value)?;
                    attributes.custom.insert("textpadding".to_string(), format!("{},{},{},{}", textpadding.0, textpadding.1, textpadding.2, textpadding.3));
                }
                "valign" => {
                    // 垂直对齐
                    attributes.custom.insert("valign".to_string(), value.to_string());
                }
                "textalign" => {
                    // 文本对齐（Label 专用）
                    attributes.custom.insert("textalign".to_string(), value.to_string());
                }
                "font" => {
                    // 字体 ID，需要查找字体映射表
                    if let Ok(font_id) = value.parse::<u32>() {
                        // 优先使用当前文件中声明的 <Font> 配置
                        let font_config = self.font_map.get(&font_id)
                            .cloned()
                            // 如果当前文件没有声明 <Font>，尝试使用全局默认表（例如 TapTap 的 install.xml）
                            .or_else(|| FontConfig::from_default_table(font_id));

                        if let Some(font_config) = font_config {
                            attributes.custom.insert("font_id".to_string(), font_config.id.to_string());
                            attributes.custom.insert("font_name".to_string(), font_config.name.clone());
                            attributes.custom.insert("font_size".to_string(), font_config.size.to_string());
                            attributes.custom.insert("font_bold".to_string(), font_config.bold.to_string());
                        } else {
                            // 找不到任何配置时，至少存储字体 ID，渲染时回退到默认字体
                            attributes.custom.insert("font_id".to_string(), font_id.to_string());
                        }
                    }
                }
                "borderround" => {
                    // 圆角格式：x,y
                    let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
                    if parts.len() == 2 {
                        if let (Ok(x), Ok(y)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>()) {
                            attributes.custom.insert("borderround".to_string(), format!("{},{}", x, y));
                        }
                    }
                }
                "bordercolor" => {
                    attributes.custom.insert("bordercolor".to_string(), value.to_string());
                }
                "bordersize" => {
                    attributes.custom.insert("bordersize".to_string(), value.to_string());
                }
                "float" => {
                    // float="true" 启用绝对定位
                    let is_float = self.parse_bool(value).unwrap_or(false);
                    attributes.custom.insert("float".to_string(), value.to_string());
                    if is_float {
                        // 如果 float=true，标记为绝对定位
                        attributes.custom.insert("is_absolute".to_string(), "true".to_string());
                    }
                }
                "pos" => {
                    // pos="x1,y1,x2,y2" 存储到 custom
                    if let Ok(pos) = self.parse_pos_rect(value) {
                        attributes.custom.insert("pos".to_string(), format!("{},{},{},{}", pos.0, pos.1, pos.2, pos.3));
                        // 同时计算 position 和尺寸（用于绝对定位）
                        attributes.position = Some((pos.0, pos.1));
                        attributes.width = Some(pos.2 - pos.0);
                        attributes.height = Some(pos.3 - pos.1);
                        // 标记为绝对定位
                        attributes.custom.insert("is_absolute".to_string(), "true".to_string());
                    }
                }
                // 图片属性（需要解析 file, dest, corner, fade）
                "normalimage" | "hotimage" | "pushedimage" | "disabledimage" | "focusedimage" |
                "normalhotimage" | "selectedimage" | "selectedhotimage" | "foreimage" => {
                    let image_path = self.parse_image_path(value);
                    // 将 ImagePath 序列化为字符串存储到 custom
                    let mut image_str = image_path.file.clone();
                    if let Some(dest) = image_path.dest {
                        image_str.push_str(&format!(" dest='{},{},{},{}'", dest.0, dest.1, dest.2, dest.3));
                    }
                    if let Some(corner) = image_path.corner {
                        image_str.push_str(&format!(" corner='{},{},{},{}'", corner.0, corner.1, corner.2, corner.3));
                    }
                    if let Some(fade) = image_path.fade {
                        image_str.push_str(&format!(" fade='{}'", fade));
                    }
                    attributes.custom.insert(key.to_string(), image_str);
                }
                // RichEdit 特有属性
                "readonly" | "autohscroll" | "wantreturn" | "wantctrlreturn" | "multiline" => {
                    attributes.custom.insert(key.to_string(), value.to_string());
                }
                // Slider 特有属性
                "min" | "max" | "value" | "thumbsize" | "mouse" => {
                    attributes.custom.insert(key.to_string(), value.to_string());
                }
                // 其他属性
                "showhtml" | "cursor" | "heigh" => {
                    // heigh 是 height 的拼写错误，修正为 height
                    if key == "heigh" {
                        attributes.height = Some(self.parse_f32(value)?);
                    } else {
                        attributes.custom.insert(key.to_string(), value.to_string());
                    }
                }
                _ => {
                    attributes.custom.insert(key.to_string(), value.to_string());
                }
            }
        }

        Ok(attributes)
    }

    /// 解析元数据
    fn parse_metadata(&self, node: &roxmltree::Node, metadata: &mut LayoutMetadata) -> Result<(), ParseError> {
        for attr in node.attributes() {
            let key = attr.name();
            let value = attr.value();

            match key {
                "name" => metadata.name = value.to_string(),
                "version" => metadata.version = value.to_string(),
                "description" => metadata.description = Some(value.to_string()),
                "author" => metadata.author = Some(value.to_string()),
                "created_at" => metadata.created_at = Some(value.to_string()),
                "modified_at" => metadata.modified_at = Some(value.to_string()),
                _ => {}
            }
        }

        Ok(())
    }

    /// 解析f32值
    fn parse_f32(&self, value: &str) -> Result<f32, ParseError> {
        value.parse::<f32>()
            .map_err(|_| ParseError::InvalidAttributeValue(format!("无效的浮点数: {}", value)))
    }

    /// 解析bool值
    fn parse_bool(&self, value: &str) -> Result<bool, ParseError> {
        match value.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Ok(true),
            "false" | "0" | "no" | "off" => Ok(false),
            _ => Err(ParseError::InvalidAttributeValue(format!("无效的布尔值: {}", value))),
        }
    }


    /// 解析位置值
    fn parse_position(&self, value: &str) -> Result<(f32, f32), ParseError> {
        let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
        if parts.len() != 2 {
            return Err(ParseError::InvalidAttributeValue(format!("无效的位置格式: {}", value)));
        }
        
        let x = self.parse_f32(parts[0])?;
        let y = self.parse_f32(parts[1])?;
        Ok((x, y))
    }

    /// 解析字体元素
    fn parse_font(&self, node: &roxmltree::Node) -> Result<FontConfig, ParseError> {
        let mut id = None;
        let mut name = None;
        let mut size = None;
        let mut bold = false;
        let mut default = false;

        for attr in node.attributes() {
            match attr.name() {
                "id" => {
                    id = Some(attr.value().parse::<u32>()
                        .map_err(|_| ParseError::InvalidAttributeValue(format!("无效的字体 ID: {}", attr.value())))?);
                }
                "name" => name = Some(attr.value().to_string()),
                "size" => {
                    size = Some(self.parse_f32(attr.value())?);
                }
                "bold" => {
                    bold = self.parse_bool(attr.value())?;
                }
                "default" => {
                    default = self.parse_bool(attr.value())?;
                }
                _ => {}
            }
        }

        Ok(FontConfig {
            id: id.ok_or_else(|| ParseError::MissingRequiredAttribute("Font 元素缺少 id 属性".to_string()))?,
            name: name.unwrap_or_else(|| "微软雅黑".to_string()),
            size: size.ok_or_else(|| ParseError::MissingRequiredAttribute("Font 元素缺少 size 属性".to_string()))?,
            bold,
            default,
        })
    }

    /// 解析窗口元数据（Window 元素）
    fn parse_window_metadata(&self, node: &roxmltree::Node, metadata: &mut LayoutMetadata) -> Result<(), ParseError> {
        for attr in node.attributes() {
            match attr.name() {
                "name" => metadata.name = attr.value().to_string(),
                "size" => {
                    // size="width,height"
                    let parts: Vec<&str> = attr.value().split(',').map(|s| s.trim()).collect();
                    if parts.len() == 2 {
                        // 存储到 metadata 的 custom 字段（如果 LayoutMetadata 有的话）
                        // 或者存储到 custom HashMap 中
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// 解析图片路径（格式：file='path' dest='x1,y1,x2,y2' corner='x1,y1,x2,y2' fade='value'）
    fn parse_image_path(&self, value: &str) -> ImagePath {
        let mut file = String::new();
        let mut dest = None;
        let mut corner = None;
        let mut fade = None;

        // 解析 file='...'
        if let Some(start) = value.find("file='") {
            let start_pos = start + 6;
            if let Some(end) = value[start_pos..].find("'") {
                file = value[start_pos..start_pos + end].to_string();
            }
        } else if let Some(start) = value.find("file=\"") {
            let start_pos = start + 6;
            if let Some(end) = value[start_pos..].find("\"") {
                file = value[start_pos..start_pos + end].to_string();
            }
        } else {
            // 如果没有 file='...'，整个值就是文件路径
            file = value.trim().to_string();
        }

        // 解析 dest='x1,y1,x2,y2'
        if let Some(start) = value.find("dest='") {
            let start_pos = start + 6;
            if let Some(end) = value[start_pos..].find("'") {
                let dest_str = &value[start_pos..start_pos + end];
                let parts: Vec<&str> = dest_str.split(',').map(|s| s.trim()).collect();
                if parts.len() == 4 {
                    if let (Ok(x1), Ok(y1), Ok(x2), Ok(y2)) = (
                        parts[0].parse::<u32>(),
                        parts[1].parse::<u32>(),
                        parts[2].parse::<u32>(),
                        parts[3].parse::<u32>(),
                    ) {
                        dest = Some((x1, y1, x2, y2));
                    }
                }
            }
        }

        // 解析 corner='x1,y1,x2,y2'
        if let Some(start) = value.find("corner='") {
            let start_pos = start + 8;
            if let Some(end) = value[start_pos..].find("'") {
                let corner_str = &value[start_pos..start_pos + end];
                let parts: Vec<&str> = corner_str.split(',').map(|s| s.trim()).collect();
                if parts.len() == 4 {
                    if let (Ok(x1), Ok(y1), Ok(x2), Ok(y2)) = (
                        parts[0].parse::<u32>(),
                        parts[1].parse::<u32>(),
                        parts[2].parse::<u32>(),
                        parts[3].parse::<u32>(),
                    ) {
                        corner = Some((x1, y1, x2, y2));
                    }
                }
            }
        }

        // 解析 fade='value'
        if let Some(start) = value.find("fade='") {
            let start_pos = start + 6;
            if let Some(end) = value[start_pos..].find("'") {
                let fade_str = &value[start_pos..start_pos + end];
                if let Ok(fade_val) = fade_str.parse::<u8>() {
                    fade = Some(fade_val);
                }
            }
        }

        ImagePath {
            file,
            dest,
            corner,
            fade,
        }
    }

    /// 解析 inset 格式（left,top,right,bottom）
    fn parse_inset(&self, value: &str) -> Result<(f32, f32, f32, f32), ParseError> {
        let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
        match parts.len() {
            1 => {
                let val = self.parse_f32(parts[0])?;
                Ok((val, val, val, val))
            }
            4 => {
                let left = self.parse_f32(parts[0])?;
                let top = self.parse_f32(parts[1])?;
                let right = self.parse_f32(parts[2])?;
                let bottom = self.parse_f32(parts[3])?;
                Ok((left, top, right, bottom))
            }
            _ => Err(ParseError::InvalidAttributeValue(format!("无效的 inset 格式: {}", value))),
        }
    }

    /// 解析 pos 格式（x1,y1,x2,y2）
    fn parse_pos_rect(&self, value: &str) -> Result<(f32, f32, f32, f32), ParseError> {
        let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
        if parts.len() != 4 {
            return Err(ParseError::InvalidAttributeValue(format!("无效的 pos 格式: {}", value)));
        }
        
        let x1 = self.parse_f32(parts[0])?;
        let y1 = self.parse_f32(parts[1])?;
        let x2 = self.parse_f32(parts[2])?;
        let y2 = self.parse_f32(parts[3])?;
        Ok((x1, y1, x2, y2))
    }
}

impl Default for XmlParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_layout() {
        let xml = r#"
        <Windows>
            <VerticalLayout>
                <Label text="Welcome" />
                <Button name="click-btn" text="Click Me" />
            </VerticalLayout>
        </Windows>
        "#;

        let mut parser = XmlParser::new();
        let result = parser.parse_string(xml);
        assert!(result.is_ok());

        let tree = result.unwrap();
        assert!(tree.has_page_element());
    }

    #[test]
    fn test_parse_with_attributes() {
        let xml = r#"
        <Windows>
            <VerticalLayout>
                <Button name="submit-btn" text="@button.submit" width="100" height="30" />
            </VerticalLayout>
        </Windows>
        "#;

        let mut parser = XmlParser::new();
        let tree = parser.parse_string(xml).unwrap();

        let button = tree.find_by_id("submit-btn");
        match button {
            Some(btn) => {
                assert_eq!(btn.attributes.text, Some("@button.submit".to_string()));
                assert_eq!(btn.attributes.width, Some(100.0));
                assert_eq!(btn.attributes.height, Some(30.0));
            }
            None => {
                panic!("Button not found");
            }
        }
    }

    #[test]
    fn test_parse_padding() {
        let xml = r#"
        <Windows>
            <VerticalLayout padding="10,20,30,40">
                <Label text="Test" />
            </VerticalLayout>
        </Windows>
        "#;

        let mut parser = XmlParser::new();
        let tree = parser.parse_string(xml).unwrap();

        let vbox = tree.find_by_type(&ElementType::VBox)[0];
        // NSIS 格式：padding="left,top,right,bottom" (10,20,30,40)
        // 转换为内部格式：top,right,bottom,left (20,30,40,10)
        assert_eq!(vbox.attributes.padding, Some((20.0, 30.0, 40.0, 10.0)));
    }

    #[test]
    fn test_parse_invalid_element_type() {
        let xml = r#"
        <Windows>
            <VerticalLayout>
                <InvalidElement text="Test" />
            </VerticalLayout>
        </Windows>
        "#;

        let mut parser = XmlParser::new();
        let result = parser.parse_string(xml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_i18n_keys() {
        let xml = r#"
        <Windows>
            <VerticalLayout>
                <Label text="@welcome.title" />
                <Button text="@button.submit" />
                <Label text="Normal text" />
            </VerticalLayout>
        </Windows>
        "#;

        let mut parser = XmlParser::new();
        let tree = parser.parse_string(xml).unwrap();

        let keys = tree.get_i18n_keys();
        assert!(keys.contains(&"welcome.title".to_string()));
        assert!(keys.contains(&"button.submit".to_string()));
        assert_eq!(keys.len(), 2);
    }

    // ── 新格式测试 ──

    #[test]
    fn test_parse_new_format_basic() {
        let xml = r##"
        <Page width="100%" height="100%" background="#FF181B22">
            <VBox width="100%" height="100%" justify-content="center" align-items="center">
                <Image id="logo" src="assets/logo.png" width="148" height="80" />
                <Button id="btnInstall" width="240" height="40" text="@install_button"
                        normal-image="assets/btn_primary.png" hover-image="assets/btn_hover.png"
                        font-size="14" color="#FFFFFFFF" />
            </VBox>
        </Page>
        "##;

        let mut parser = XmlParser::new();
        let tree = parser.parse_string(xml).unwrap();

        assert!(tree.has_page_element());

        // VBox should have correct flex properties
        let vboxes = tree.find_by_type(&ElementType::VBox);
        assert_eq!(vboxes.len(), 1);
        let vbox = vboxes[0];
        let fs = vbox.flex_style.as_ref().unwrap();
        assert_eq!(fs.flex_direction, FlexDirection::Column);
        assert_eq!(fs.justify_content, JustifyContent::Center);
        assert_eq!(fs.align_items, AlignItems::Center);
        assert_eq!(fs.width, Dimension::Percent(100.0));

        // Button should have widget props
        let btn = tree.find_by_id("btnInstall").unwrap();
        let wp = btn.widget_props.as_ref().unwrap();
        assert_eq!(wp.text, Some("@install_button".to_string()));
        assert_eq!(wp.normal_image, Some("assets/btn_primary.png".to_string()));
        assert!(wp.is_i18n_text());
        assert_eq!(wp.get_i18n_key(), Some("install_button"));

        // Button visual style
        let vs = btn.visual_style.as_ref().unwrap();
        assert_eq!(vs.font_size, Some(14.0));
        assert_eq!(vs.color, Some("#FFFFFFFF".to_string()));

        // Image should have src
        let img = tree.find_by_id("logo").unwrap();
        let img_wp = img.widget_props.as_ref().unwrap();
        assert_eq!(img_wp.src, Some("assets/logo.png".to_string()));
    }

    #[test]
    fn test_parse_new_format_absolute_positioning() {
        let xml = r#"
        <Page width="100%" height="100%">
            <Overlay id="panel" position="absolute" left="0" top="358" width="100%" height="160"
                     background-image="assets/bg_color.png" visible="false">
                <Label id="lbl" text="Hello" />
            </Overlay>
        </Page>
        "#;

        let mut parser = XmlParser::new();
        let tree = parser.parse_string(xml).unwrap();

        let overlay = tree.find_by_id("panel").unwrap();
        let fs = overlay.flex_style.as_ref().unwrap();
        assert_eq!(fs.position, Position::Absolute);
        assert_eq!(fs.left, Dimension::Px(0.0));
        assert_eq!(fs.top, Dimension::Px(358.0));
        assert_eq!(fs.width, Dimension::Percent(100.0));
        assert_eq!(fs.height, Dimension::Px(160.0));

        let vs = overlay.visual_style.as_ref().unwrap();
        assert!(!vs.visible);
        assert_eq!(vs.background_image, Some("assets/bg_color.png".to_string()));

        // Legacy compat
        assert_eq!(overlay.attributes.custom.get("is_absolute"), Some(&"true".to_string()));
    }

    #[test]
    fn test_parse_new_format_margin_padding() {
        let xml = r##"
        <Page width="574" height="358">
            <HBox id="hbox" width="100%" height="87" padding="40 35" align-items="center">
                <Checkbox id="chk" width="16" height="16"
                          unchecked-image="assets/checkbox-0.png" checked-image="assets/checkbox-2.png" />
                <Label id="lbl" text="@agree_prefix" color="#FFE4E8EC" font-size="12" margin-left="4" />
            </HBox>
        </Page>
        "##;

        let mut parser = XmlParser::new();
        let tree = parser.parse_string(xml).unwrap();

        let hbox = tree.find_by_id("hbox").unwrap();
        let fs = hbox.flex_style.as_ref().unwrap();
        // padding="40 35" -> top/bottom=40, left/right=35
        assert_eq!(fs.padding.top, Dimension::Px(40.0));
        assert_eq!(fs.padding.right, Dimension::Px(35.0));
        assert_eq!(fs.padding.bottom, Dimension::Px(40.0));
        assert_eq!(fs.padding.left, Dimension::Px(35.0));

        let lbl = tree.find_by_id("lbl").unwrap();
        let lfs = lbl.flex_style.as_ref().unwrap();
        assert_eq!(lfs.margin.left, Dimension::Px(4.0));
        assert_eq!(lfs.margin.top, Dimension::Px(0.0)); // default 0

        // Checkbox should have unchecked/checked images
        let chk = tree.find_by_id("chk").unwrap();
        let wp = chk.widget_props.as_ref().unwrap();
        assert_eq!(wp.unchecked_image, Some("assets/checkbox-0.png".to_string()));
        assert_eq!(wp.checked_image, Some("assets/checkbox-2.png".to_string()));
    }

    #[test]
    fn test_parse_new_format_text_content() {
        let xml = r#"
        <Page width="574" height="358">
            <Button id="btn" width="240" height="40"
                    normal-image="assets/btn.png">
                @install_button
            </Button>
        </Page>
        "#;

        let mut parser = XmlParser::new();
        let tree = parser.parse_string(xml).unwrap();

        let btn = tree.find_by_id("btn").unwrap();
        // text from inner content
        assert_eq!(btn.attributes.text, Some("@install_button".to_string()));
        let wp = btn.widget_props.as_ref().unwrap();
        assert_eq!(wp.text, Some("@install_button".to_string()));
        assert!(wp.is_i18n_text());
    }

    #[test]
    fn test_parse_new_format_flex_grow() {
        let xml = r#"
        <Page width="100%" height="100%">
            <HBox width="100%" height="40">
                <Label id="fixed" text="F" width="100" height="40" />
                <Spacer flex-grow="1" />
                <Label id="end" text="E" width="100" height="40" />
            </HBox>
        </Page>
        "#;

        let mut parser = XmlParser::new();
        let tree = parser.parse_string(xml).unwrap();

        let spacers = tree.find_by_type(&ElementType::Spacer);
        assert_eq!(spacers.len(), 1);
        let spacer = spacers[0];
        let fs = spacer.flex_style.as_ref().unwrap();
        assert_eq!(fs.flex_grow, 1.0);
    }
}
