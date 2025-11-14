//! XML布局解析器
//! 
//! 解析简化的XML布局格式并生成内部布局树

use crate::layout::{LayoutTree, LayoutElement, ElementType, ElementAttributes};
use crate::layout::layout_tree::LayoutMetadata;
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
        
        // 支持 <Layout> 和 <Windows>/<Window> 作为根元素
        let (page_node, mut metadata) = match root_name {
            "Layout" => {
                // 旧格式：<Layout> -> <Page>
                let mut metadata = LayoutMetadata::default();
                self.parse_metadata(&root, &mut metadata)?;
                
                let page_node = root.children()
                    .find(|n| n.is_element() && n.tag_name().name() == "Page")
                    .ok_or_else(|| ParseError::ValidationFailed(vec!["缺少 <Page> 元素".to_string()]))?;
                
                (page_node, metadata)
            }
            "Windows" | "Window" => {
                // 新格式：<Windows> 或 <Window> -> 直接包含布局元素
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
                
                // 创建一个虚拟的 Page 元素来包装布局
                (layout_node, metadata)
            }
            _ => {
                return Err(ParseError::XmlError(format!("不支持的根元素: {}", root_name)));
            }
        };
        
        // 解析布局元素
        let page_element = self.parse_element(&page_node)?;
        
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
        
        let mut element = LayoutElement::with_attributes(element_type, attributes);
        
        // 递归解析子元素
        for child in node.children() {
            if child.is_element() {
                let child_element = self.parse_element(&child)?;
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

    /// 解析元素类型
    fn parse_element_type(&self, name: &str) -> Result<ElementType, ParseError> {
        match name {
            // 旧格式元素（保留兼容）
            "Page" => Ok(ElementType::Page),
            "VBox" => Ok(ElementType::VBox),
            "HBox" => Ok(ElementType::HBox),
            "Spacer" => Ok(ElementType::Spacer),
            "Flex" => Ok(ElementType::Flex),
            "Button" => Ok(ElementType::Button),
            "Label" => Ok(ElementType::Label),
            "Checkbox" => Ok(ElementType::Checkbox),
            "TextInput" => Ok(ElementType::TextInput),
            "Image" => Ok(ElementType::Image),
            "ProgressBar" => Ok(ElementType::ProgressBar),
            "Divider" => Ok(ElementType::Divider),
            "Overlay" => Ok(ElementType::Overlay),
            // 新格式元素（标准布局格式）
            "Windows" => Ok(ElementType::Page),  // 根元素，映射为 Page
            "Window" => Ok(ElementType::Page),   // 窗口元素，映射为 Page
            "VerticalLayout" => Ok(ElementType::VBox),
            "HorizontalLayout" => Ok(ElementType::HBox),
            "Container" => Ok(ElementType::Spacer),
            "Control" => Ok(ElementType::Spacer),
            "CheckBox" => Ok(ElementType::Checkbox),  // 支持 CheckBox（NSIS 格式）
            "RichEdit" => Ok(ElementType::TextInput),
            "Slider" => Ok(ElementType::ProgressBar),
            "TabLayout" => Ok(ElementType::VBox),  // 暂时映射为 VBox
            "Include" => Err(ParseError::InvalidElementType("Include 需要特殊处理".to_string())),
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
                // 旧格式属性（保留兼容）
                "id" => attributes.id = Some(value.to_string()),
                "text" => attributes.text = Some(value.to_string()),
                "text_i18n" => attributes.text_i18n = Some(value.to_string()),
                "width" => attributes.width = Some(self.parse_f32(value)?),
                "height" => attributes.height = Some(self.parse_f32(value)?),
                "min_width" => attributes.min_width = Some(self.parse_f32(value)?),
                "max_width" => attributes.max_width = Some(self.parse_f32(value)?),
                "padding" => {
                    // 新格式：padding="left,top,right,bottom"，需要转换
                    let padding = self.parse_inset(value)?;  // left,top,right,bottom
                    // 转换为 top,right,bottom,left
                    attributes.padding = Some((padding.1, padding.2, padding.3, padding.0));
                }
                "spacing" => attributes.spacing = Some(self.parse_f32(value)?),
                "background" => attributes.background = Some(value.to_string()),
                "color" => attributes.color = Some(value.to_string()),
                "style" => attributes.style = Some(value.to_string()),
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
                "font_size" => {
                    attributes.custom.insert("font_size".to_string(), value.to_string());
                }
                // 新格式属性（标准布局格式）
                "name" => {
                    // name 映射到 id
                    attributes.id = Some(value.to_string());
                }
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
                        if let Some(font_config) = self.font_map.get(&font_id) {
                            attributes.custom.insert("font_id".to_string(), font_id.to_string());
                            attributes.custom.insert("font_name".to_string(), font_config.name.clone());
                            attributes.custom.insert("font_size".to_string(), font_config.size.to_string());
                            attributes.custom.insert("font_bold".to_string(), font_config.bold.to_string());
                        } else {
                            // 字体 ID 未找到，存储原始值
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

    /// 解析内边距值
    fn parse_padding(&self, value: &str) -> Result<(f32, f32, f32, f32), ParseError> {
        let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
        match parts.len() {
            1 => {
                let val = self.parse_f32(parts[0])?;
                Ok((val, val, val, val))
            }
            2 => {
                let top_bottom = self.parse_f32(parts[0])?;
                let left_right = self.parse_f32(parts[1])?;
                Ok((top_bottom, left_right, top_bottom, left_right))
            }
            4 => {
                let top = self.parse_f32(parts[0])?;
                let right = self.parse_f32(parts[1])?;
                let bottom = self.parse_f32(parts[2])?;
                let left = self.parse_f32(parts[3])?;
                Ok((top, right, bottom, left))
            }
            _ => Err(ParseError::InvalidAttributeValue(format!("无效的内边距格式: {}", value))),
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
        <Layout name="Test Layout" version="1.0.0">
            <Page>
                <VBox>
                    <Label text="Welcome" />
                    <Button text="Click Me" style="primary" />
                </VBox>
            </Page>
        </Layout>
        "#;

        let parser = XmlParser::new();
        let result = parser.parse_string(xml);
        assert!(result.is_ok());

        let tree = result.unwrap();
        assert_eq!(tree.metadata.name, "Test Layout");
        assert_eq!(tree.metadata.version, "1.0.0");
        assert!(tree.has_page_element());
    }

    #[test]
    fn test_parse_with_attributes() {
        let xml = r#"
        <Layout>
            <Page>
                <Button id="submit-btn" text="@button.submit" width="100" height="30" style="primary" />
            </Page>
        </Layout>
        "#;

        let mut parser = XmlParser::new();
        let tree = parser.parse_string(xml).unwrap();

        let button = tree.find_by_id("submit-btn");
        match button {
            Some(btn) => {
                assert_eq!(btn.attributes.text, Some("@button.submit".to_string()));
                assert_eq!(btn.attributes.width, Some(100.0));
                assert_eq!(btn.attributes.height, Some(30.0));
                assert_eq!(btn.attributes.style, Some("primary".to_string()));
            }
            None => {
                panic!("Button not found");
            }
        }
    }

    #[test]
    fn test_parse_padding() {
        let xml = r#"
        <Layout>
            <Page>
                <VBox padding="10,20,30,40">
                    <Label text="Test" />
                </VBox>
            </Page>
        </Layout>
        "#;

        let mut parser = XmlParser::new();
        let tree = parser.parse_string(xml).unwrap();

        let vbox = tree.find_by_type(&ElementType::VBox)[0];
        assert_eq!(vbox.attributes.padding, Some((10.0, 20.0, 30.0, 40.0)));
    }

    #[test]
    fn test_parse_invalid_element_type() {
        let xml = r#"
        <Layout>
            <Page>
                <InvalidElement text="Test" />
            </Page>
        </Layout>
        "#;

        let mut parser = XmlParser::new();
        let result = parser.parse_string(xml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_i18n_keys() {
        let xml = r#"
        <Layout>
            <Page>
                <Label text="@welcome.title" />
                <Button text="@button.submit" />
                <Label text="Normal text" />
            </Page>
        </Layout>
        "#;

        let mut parser = XmlParser::new();
        let tree = parser.parse_string(xml).unwrap();

        let keys = tree.get_i18n_keys();
        assert!(keys.contains(&"welcome.title".to_string()));
        assert!(keys.contains(&"button.submit".to_string()));
        assert_eq!(keys.len(), 2);
    }
}
