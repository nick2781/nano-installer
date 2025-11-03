//! 布局树定义
//! 
//! 定义内部布局树结构和操作

use crate::layout::element::{LayoutElement, ElementType};
use std::collections::HashMap;

/// 布局树根节点
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutTree {
    /// 根元素
    pub root: LayoutElement,
    /// 元数据
    pub metadata: LayoutMetadata,
}

/// 布局元数据
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutMetadata {
    /// 布局名称
    pub name: String,
    /// 布局版本
    pub version: String,
    /// 布局描述
    pub description: Option<String>,
    /// 作者
    pub author: Option<String>,
    /// 创建时间
    pub created_at: Option<String>,
    /// 最后修改时间
    pub modified_at: Option<String>,
    /// 依赖的资源文件
    pub dependencies: Vec<String>,
}

impl Default for LayoutMetadata {
    fn default() -> Self {
        Self {
            name: "Untitled Layout".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            author: None,
            created_at: None,
            modified_at: None,
            dependencies: Vec::new(),
        }
    }
}

/// 布局节点（用于遍历）
#[derive(Debug, Clone)]
pub struct LayoutNode<'a> {
    /// 元素引用
    pub element: &'a LayoutElement,
    /// 父节点引用
    pub parent: Option<&'a LayoutNode<'a>>,
    /// 深度
    pub depth: usize,
    /// 路径
    pub path: String,
}

impl LayoutTree {
    /// 创建新的布局树
    pub fn new(root: LayoutElement) -> Self {
        Self {
            root,
            metadata: LayoutMetadata::default(),
        }
    }

    /// 创建带元数据的布局树
    pub fn with_metadata(root: LayoutElement, metadata: LayoutMetadata) -> Self {
        Self { root, metadata }
    }

    /// 验证整个布局树
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // 验证根元素
        if let Err(root_errors) = self.root.validate() {
            errors.extend(root_errors);
        }

        // 验证必需元素
        if !self.has_page_element() {
            errors.push("布局树必须包含至少一个 Page 元素".to_string());
        }

        // 验证资源依赖
        for _dependency in &self.metadata.dependencies {
            // 这里可以添加资源存在性检查
            // 暂时跳过，在解析阶段处理
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// 检查是否包含页面元素
    pub fn has_page_element(&self) -> bool {
        self.root.element_type == ElementType::Page || 
        !self.root.find_by_type(&ElementType::Page).is_empty()
    }

    /// 获取所有页面元素
    pub fn get_pages(&self) -> Vec<&LayoutElement> {
        if self.root.element_type == ElementType::Page {
            vec![&self.root]
        } else {
            self.root.find_by_type(&ElementType::Page)
        }
    }

    /// 获取所有按钮元素
    pub fn get_buttons(&self) -> Vec<&LayoutElement> {
        self.root.find_by_type(&ElementType::Button)
    }

    /// 获取所有输入元素
    pub fn get_inputs(&self) -> Vec<&LayoutElement> {
        let mut inputs = Vec::new();
        inputs.extend(self.root.find_by_type(&ElementType::TextInput));
        inputs.extend(self.root.find_by_type(&ElementType::Checkbox));
        inputs
    }

    /// 获取所有文本元素
    pub fn get_text_elements(&self) -> Vec<&LayoutElement> {
        self.root.find_by_type(&ElementType::Label)
    }

    /// 按ID查找元素
    pub fn find_by_id(&self, id: &str) -> Option<&LayoutElement> {
        self.root.find_by_id(id)
    }

    /// 按类型查找元素
    pub fn find_by_type(&self, element_type: &ElementType) -> Vec<&LayoutElement> {
        self.root.find_by_type(element_type)
    }

    /// 获取所有国际化字符串键
    pub fn get_i18n_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        self.collect_i18n_keys(&self.root, &mut keys);
        keys
    }

    /// 递归收集国际化键
    fn collect_i18n_keys(&self, element: &LayoutElement, keys: &mut Vec<String>) {
        if let Some(key) = element.attributes.get_i18n_key() {
            keys.push(key.to_string());
        }

        for child in &element.children {
            self.collect_i18n_keys(child, keys);
        }
    }

    /// 获取所有资源依赖
    pub fn get_resource_dependencies(&self) -> Vec<String> {
        let mut resources = Vec::new();
        self.collect_resources(&self.root, &mut resources);
        resources
    }

    /// 递归收集资源
    fn collect_resources(&self, element: &LayoutElement, resources: &mut Vec<String>) {
        // 收集图片资源
        if let Some(icon) = &element.attributes.icon {
            resources.push(icon.clone());
        }

        // 收集背景图片
        if let Some(background) = &element.attributes.background {
            if background.starts_with("assets/") || background.ends_with(".png") || background.ends_with(".jpg") {
                resources.push(background.clone());
            }
        }

        for child in &element.children {
            self.collect_resources(child, resources);
        }
    }

    /// 遍历所有节点
    pub fn walk_nodes<F>(&self, mut visitor: F)
    where
        F: FnMut(&LayoutNode),
    {
        self.walk_node_recursive(&self.root, None, 0, "", &mut visitor);
    }

    /// 递归遍历节点
    fn walk_node_recursive<F>(
        &self,
        element: &LayoutElement,
        parent: Option<&LayoutNode>,
        depth: usize,
        path: &str,
        visitor: &mut F,
    ) where
        F: FnMut(&LayoutNode),
    {
        let current_path = if path.is_empty() {
            element.attributes.id.as_ref().map_or("root", |id| id.as_str()).to_string()
        } else {
            format!("{}/{}", path, element.attributes.id.as_ref().map_or("unnamed", |id| id.as_str()))
        };

        let node = LayoutNode {
            element,
            parent,
            depth,
            path: current_path.clone(),
        };

        visitor(&node);

        for child in &element.children {
            self.walk_node_recursive(child, Some(&node), depth + 1, &current_path, visitor);
        }
    }

    /// 获取布局统计信息
    pub fn get_stats(&self) -> LayoutStats {
        let mut stats = LayoutStats::default();
        self.collect_stats(&self.root, &mut stats);
        stats
    }

    /// 递归收集统计信息
    fn collect_stats(&self, element: &LayoutElement, stats: &mut LayoutStats) {
        stats.total_elements += 1;

        match element.element_type {
            ElementType::Page => stats.pages += 1,
            ElementType::VBox => stats.vboxes += 1,
            ElementType::HBox => stats.hboxes += 1,
            ElementType::Button => stats.buttons += 1,
            ElementType::Label => stats.labels += 1,
            ElementType::Checkbox => stats.checkboxes += 1,
            ElementType::TextInput => stats.text_inputs += 1,
            ElementType::Image => stats.images += 1,
            ElementType::ProgressBar => stats.progress_bars += 1,
            ElementType::Divider => stats.dividers += 1,
            ElementType::Overlay => stats.overlays += 1,
            ElementType::Spacer => stats.spacers += 1,
            ElementType::Flex => stats.flexes += 1,
        }

        if element.attributes.is_i18n_text() {
            stats.i18n_texts += 1;
        }

        if element.attributes.icon.is_some() {
            stats.images += 1;
        }

        for child in &element.children {
            self.collect_stats(child, stats);
        }
    }
}

/// 布局统计信息
#[derive(Debug, Clone, Default)]
pub struct LayoutStats {
    /// 总元素数
    pub total_elements: usize,
    /// 页面数
    pub pages: usize,
    /// 垂直布局数
    pub vboxes: usize,
    /// 水平布局数
    pub hboxes: usize,
    /// 按钮数
    pub buttons: usize,
    /// 标签数
    pub labels: usize,
    /// 复选框数
    pub checkboxes: usize,
    /// 文本输入框数
    pub text_inputs: usize,
    /// 图片数
    pub images: usize,
    /// 进度条数
    pub progress_bars: usize,
    /// 分隔线数
    pub dividers: usize,
    /// 浮动层数
    pub overlays: usize,
    /// 空白占位数
    pub spacers: usize,
    /// 弹性空间数
    pub flexes: usize,
    /// 国际化文本数
    pub i18n_texts: usize,
}

impl LayoutStats {
    /// 获取元素类型分布
    pub fn get_element_distribution(&self) -> HashMap<String, usize> {
        let mut dist = HashMap::new();
        dist.insert("Page".to_string(), self.pages);
        dist.insert("VBox".to_string(), self.vboxes);
        dist.insert("HBox".to_string(), self.hboxes);
        dist.insert("Button".to_string(), self.buttons);
        dist.insert("Label".to_string(), self.labels);
        dist.insert("Checkbox".to_string(), self.checkboxes);
        dist.insert("TextInput".to_string(), self.text_inputs);
        dist.insert("Image".to_string(), self.images);
        dist.insert("ProgressBar".to_string(), self.progress_bars);
        dist.insert("Divider".to_string(), self.dividers);
        dist.insert("Overlay".to_string(), self.overlays);
        dist.insert("Spacer".to_string(), self.spacers);
        dist.insert("Flex".to_string(), self.flexes);
        dist
    }

    /// 获取国际化覆盖率
    pub fn get_i18n_coverage(&self) -> f32 {
        if self.labels == 0 {
            0.0
        } else {
            self.i18n_texts as f32 / self.labels as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::element::{ElementAttributes, ElementType};

    #[test]
    fn test_layout_tree_creation() {
        let root = LayoutElement::new(ElementType::Page);
        let tree = LayoutTree::new(root);
        assert_eq!(tree.metadata.name, "Untitled Layout");
        assert_eq!(tree.metadata.version, "1.0.0");
    }

    #[test]
    fn test_layout_tree_validation() {
        let root = LayoutElement::new(ElementType::Page);
        let tree = LayoutTree::new(root);
        assert!(tree.validate().is_ok());
        assert!(tree.has_page_element());
    }

    #[test]
    fn test_layout_tree_find_elements() {
        let root = LayoutElement::new(ElementType::Page)
            .add_child(
                LayoutElement::with_attributes(
                    ElementType::VBox,
                    ElementAttributes::new().with_id("main-container"),
                )
                .add_child(
                    LayoutElement::with_attributes(
                        ElementType::Button,
                        ElementAttributes::new().with_id("submit-button").with_text("Submit"),
                    ),
                )
                .add_child(
                    LayoutElement::with_attributes(
                        ElementType::Label,
                        ElementAttributes::new().with_text("@welcome.title"),
                    ),
                ),
            );

        let tree = LayoutTree::new(root);

        assert!(tree.find_by_id("main-container").is_some());
        assert!(tree.find_by_id("submit-button").is_some());
        assert!(tree.find_by_id("nonexistent").is_none());

        let buttons = tree.get_buttons();
        assert_eq!(buttons.len(), 1);

        let labels = tree.get_text_elements();
        assert_eq!(labels.len(), 1);
    }

    #[test]
    fn test_i18n_keys_collection() {
        let root = LayoutElement::new(ElementType::Page)
            .add_child(
                LayoutElement::with_attributes(
                    ElementType::Label,
                    ElementAttributes::new().with_text("@welcome.title"),
                ),
            )
            .add_child(
                LayoutElement::with_attributes(
                    ElementType::Button,
                    ElementAttributes::new().with_text("@button.submit"),
                ),
            );

        let tree = LayoutTree::new(root);
        let keys = tree.get_i18n_keys();
        
        assert!(keys.contains(&"welcome.title".to_string()));
        assert!(keys.contains(&"button.submit".to_string()));
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_resource_dependencies() {
        let root = LayoutElement::new(ElementType::Page)
            .add_child(
                LayoutElement::with_attributes(
                    ElementType::Image,
                    ElementAttributes::new().with_icon("assets/logo.png"),
                ),
            )
            .add_child(
                LayoutElement::with_attributes(
                    ElementType::Button,
                    ElementAttributes::new()
                        .with_text("Click Me")
                        .with_background("assets/btn_primary.png"),
                ),
            );

        let tree = LayoutTree::new(root);
        let resources = tree.get_resource_dependencies();
        
        assert!(resources.contains(&"assets/logo.png".to_string()));
        assert!(resources.contains(&"assets/btn_primary.png".to_string()));
    }

    #[test]
    fn test_layout_stats() {
        let root = LayoutElement::new(ElementType::Page)
            .add_child(LayoutElement::new(ElementType::VBox))
            .add_child(LayoutElement::new(ElementType::Button))
            .add_child(LayoutElement::new(ElementType::Label));

        let tree = LayoutTree::new(root);
        let stats = tree.get_stats();

        assert_eq!(stats.total_elements, 4); // Page + VBox + Button + Label
        assert_eq!(stats.pages, 1);
        assert_eq!(stats.vboxes, 1);
        assert_eq!(stats.buttons, 1);
        assert_eq!(stats.labels, 1);
    }
}
