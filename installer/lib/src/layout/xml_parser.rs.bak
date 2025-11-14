//! XML布局解析器
//! 
//! 解析简化的XML布局格式并生成内部布局树

use crate::layout::{LayoutTree, LayoutElement, ElementType, ElementAttributes};
use crate::layout::layout_tree::LayoutMetadata;
use roxmltree::Document;

/// XML解析器
pub struct XmlParser {
    /// 解析选项
    options: ParserOptions,
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
        }
    }

    /// 创建带选项的解析器
    pub fn with_options(options: ParserOptions) -> Self {
        Self { options }
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
    pub fn parse_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<LayoutTree, ParseError> {
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
    pub fn parse_string(&self, xml: &str) -> Result<LayoutTree, ParseError> {
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
        
        // 检查根元素是否为 Layout
        if root.tag_name().name() != "Layout" {
            return Err(ParseError::XmlError("根元素必须是 <Layout>".to_string()));
        }
        
        // 解析元数据
        let mut metadata = LayoutMetadata::default();
        self.parse_metadata(&root, &mut metadata)?;
        
        // 查找 Page 元素
        let page_node = root.children()
            .find(|n| n.is_element() && n.tag_name().name() == "Page")
            .ok_or_else(|| ParseError::ValidationFailed(vec!["缺少 <Page> 元素".to_string()]))?;
        
        // 解析 Page 元素及其子元素
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
                "id" => attributes.id = Some(value.to_string()),
                "text" => attributes.text = Some(value.to_string()),
                "text_i18n" => attributes.text_i18n = Some(value.to_string()),
                "width" => attributes.width = Some(self.parse_f32(value)?),
                "height" => attributes.height = Some(self.parse_f32(value)?),
                "min_width" => attributes.min_width = Some(self.parse_f32(value)?),
                "max_width" => attributes.max_width = Some(self.parse_f32(value)?),
                "padding" => attributes.padding = Some(self.parse_padding(value)?),
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
                    // font_size 作为自定义属性存储
                    attributes.custom.insert("font_size".to_string(), value.to_string());
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

        let parser = XmlParser::new();
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

        let parser = XmlParser::new();
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

        let parser = XmlParser::new();
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

        let parser = XmlParser::new();
        let tree = parser.parse_string(xml).unwrap();

        let keys = tree.get_i18n_keys();
        assert!(keys.contains(&"welcome.title".to_string()));
        assert!(keys.contains(&"button.submit".to_string()));
        assert_eq!(keys.len(), 2);
    }
}
