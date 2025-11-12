//! 布局渲染器
//! 
//! 将布局树渲染到egui UI

use egui::{Ui, Response, Rect, Vec2, Pos2, Color32, CornerRadius, Align};
use crate::layout::{LayoutTree, LayoutElement, ElementType, ElementAttributes};
use crate::ui::{dpi_handler::DpiConfig, style_engine::{StyleEngine, StyleType}};
use std::collections::HashMap;

/// 布局渲染器
pub struct LayoutRenderer {
    /// DPI配置
    dpi_config: DpiConfig,
    /// 样式引擎
    style_engine: StyleEngine,
    /// 资源缓存
    resource_cache: crate::ui::dpi_handler::ResourceCache,
    /// 国际化字符串
    i18n_strings: HashMap<String, String>,
    /// 交互状态
    interaction_state: InteractionState,
}

/// 交互状态
#[derive(Debug, Default)]
pub struct InteractionState {
    /// 按钮点击状态
    button_clicks: HashMap<String, bool>,
    /// 复选框状态
    checkbox_states: HashMap<String, bool>,
    /// 文本输入内容
    text_inputs: HashMap<String, String>,
    /// 悬停状态
    hover_states: HashMap<String, bool>,
}

/// 图片路径和裁剪信息
#[derive(Debug, Clone)]
struct ImagePath {
    path: String,
    dest: Option<(f32, f32, f32, f32)>, // (x1, y1, x2, y2) 裁剪区域
}

impl LayoutRenderer {
    /// 解析图片路径，支持 NSIS 格式：file='path' dest='x1,y1,x2,y2'
    fn parse_image_path(image_attr: &str) -> ImagePath {
        // 如果包含 file=' 和 dest='，解析 NSIS 格式
        if image_attr.contains("file='") && image_attr.contains("dest='") {
            // 提取 file='...' 部分
            let file_start = image_attr.find("file='").unwrap_or(0) + 6;
            let file_end = image_attr[file_start..].find("'").unwrap_or(image_attr.len() - file_start);
            let path = image_attr[file_start..file_start + file_end].to_string();
            
            // 提取 dest='...' 部分
            let dest_start = image_attr.find("dest='").unwrap_or(0) + 6;
            let dest_end = image_attr[dest_start..].find("'").unwrap_or(image_attr.len() - dest_start);
            let dest_str = image_attr[dest_start..dest_start + dest_end].to_string();
            
            // 解析 dest='x1,y1,x2,y2'
            let dest = dest_str.split(',')
                .map(|s| s.trim().parse::<f32>().ok())
                .collect::<Vec<_>>();
            
            if dest.len() == 4 && dest.iter().all(|x| x.is_some()) {
                ImagePath {
                    path,
                    dest: Some((dest[0].unwrap(), dest[1].unwrap(), dest[2].unwrap(), dest[3].unwrap())),
                }
            } else {
                ImagePath {
                    path,
                    dest: None,
                }
            }
        } else {
            // 普通路径格式
            ImagePath {
                path: image_attr.to_string(),
                dest: None,
            }
        }
    }

    /// 创建新的布局渲染器
    pub fn new(dpi_config: DpiConfig, i18n_strings: HashMap<String, String>) -> Self {
        Self {
            dpi_config,
            style_engine: StyleEngine::new(),
            resource_cache: crate::ui::dpi_handler::ResourceCache::new(),
            i18n_strings,
            interaction_state: InteractionState::default(),
        }
    }

    /// 创建带样式引擎的布局渲染器
    pub fn with_style_engine(
        dpi_config: DpiConfig, 
        style_engine: StyleEngine, 
        i18n_strings: HashMap<String, String>
    ) -> Self {
        Self {
            dpi_config,
            style_engine,
            resource_cache: crate::ui::dpi_handler::ResourceCache::new(),
            i18n_strings,
            interaction_state: InteractionState::default(),
        }
    }

    /// 渲染布局树
    pub fn render(&mut self, ui: &mut Ui, layout_tree: &LayoutTree) -> RenderResult {
        let mut result = RenderResult::new();
        
        // 应用全局样式
        self.style_engine.apply_global_style(ui.style_mut());
        
        // 渲染根元素
        self.render_element(ui, &layout_tree.root, &mut result);
        
        result
    }

    /// 渲染单个元素
    fn render_element(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        match &element.element_type {
            ElementType::Page => self.render_page(ui, element, result),
            ElementType::VBox => self.render_vbox(ui, element, result),
            ElementType::HBox => self.render_hbox(ui, element, result),
            ElementType::Spacer => self.render_spacer(ui, element, result),
            ElementType::Flex => self.render_flex(ui, element, result),
            ElementType::Button => self.render_button(ui, element, result),
            ElementType::Label => self.render_label(ui, element, result),
            ElementType::Checkbox => self.render_checkbox(ui, element, result),
            ElementType::TextInput => self.render_text_input(ui, element, result),
            ElementType::Image => self.render_image(ui, element, result),
            ElementType::ProgressBar => self.render_progress_bar(ui, element, result),
            ElementType::Divider => self.render_divider(ui, element, result),
            ElementType::Overlay => self.render_overlay(ui, element, result),
        }
    }

    /// 渲染页面
    fn render_page(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        // 移除频繁的日志输出，避免每帧都输出

        // 获取窗口尺寸（从 XML 或使用 DpiConfig 的值，确保与窗口大小一致）
        let page_width = element.attributes.width.unwrap_or(self.dpi_config.window_width);
        let page_height = element.attributes.height.unwrap_or(self.dpi_config.window_height);

        // 先分配整个页面的矩形空间
        let page_rect = ui.max_rect();
        let (full_rect, _response) = ui.allocate_exact_size(
            egui::vec2(page_width, page_height),
            egui::Sense::hover()
        );

        // 如果有背景图片，在底层渲染背景（填充整个窗口，而不是只填充页面矩形）
        if let Some(background) = &element.attributes.background {
            // 记录背景路径和窗口大小（只在首次加载时输出）
            use std::sync::Mutex;
            use once_cell::sync::Lazy;
            static BACKGROUND_LOADED: Lazy<Mutex<std::collections::HashSet<String>>> = Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
            let mut loaded = BACKGROUND_LOADED.lock().unwrap();
            if !loaded.contains(background) {
                eprintln!("[背景] 加载背景图片: {}", background);
                eprintln!("[背景]   use_2x: {}, 窗口大小: {}x{}", 
                    self.dpi_config.use_2x, 
                    self.dpi_config.window_width, 
                    self.dpi_config.window_height);
                loaded.insert(background.clone());
            }
            drop(loaded);
            
            if let Some(texture) = self.resource_cache.get_background(ui.ctx(), &self.dpi_config, background) {
                let texture_size = texture.size();
                let window_rect = ui.max_rect();
                
                // 只在首次成功加载时输出
                static BACKGROUND_SUCCESS: Lazy<Mutex<std::collections::HashSet<String>>> = Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
                let mut success = BACKGROUND_SUCCESS.lock().unwrap();
                if !success.contains(background) {
                    eprintln!("[背景] ✓ 背景图片加载成功: {} ({}x{}), 窗口: {}x{}", 
                        background, texture_size[0], texture_size[1],
                        window_rect.width(), window_rect.height());
                    
                    // 检查图片尺寸和窗口尺寸是否匹配
                    let width_match = (texture_size[0] as f32 - window_rect.width()).abs() < 1.0;
                    let height_match = (texture_size[1] as f32 - window_rect.height()).abs() < 1.0;
                    if !width_match || !height_match {
                        eprintln!("[背景] ⚠️  警告: 背景图片尺寸与窗口大小不匹配！");
                        eprintln!("[背景]   图片: {}x{}, 窗口: {}x{}, 差异: {}x{}",
                            texture_size[0], texture_size[1],
                            window_rect.width(), window_rect.height(),
                            (texture_size[0] as f32 - window_rect.width()).abs(),
                            (texture_size[1] as f32 - window_rect.height()).abs());
                    } else {
                        eprintln!("[背景] ✓ 背景图片尺寸与窗口大小匹配");
                    }
                    success.insert(background.clone());
                }
                drop(success);
                
                // 使用整个窗口矩形作为背景，而不是只填充页面矩形
                ui.painter().image(
                    texture.id(),
                    window_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            } else {
                static BACKGROUND_FAILED: Lazy<Mutex<std::collections::HashSet<String>>> = Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
                let mut failed = BACKGROUND_FAILED.lock().unwrap();
                if !failed.contains(background) {
                    eprintln!("[背景] ✗ 背景图片加载失败: {}", background);
                    failed.insert(background.clone());
                }
            }
        }

        // 使用绝对定位渲染（计算累积的 Y 坐标，相对于 full_rect.min）
        let mut current_y = full_rect.min.y;

        // 创建一个子 UI，限制在整个页面矩形内
        let mut page_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(full_rect)
                .layout(egui::Layout::top_down(egui::Align::Min))
        );

        for child in &element.children {
            // 渲染子元素并更新 Y 坐标
            current_y = self.render_element_at_y_absolute(&mut page_ui, child, current_y, full_rect, result);
        }
    }

    /// 在绝对 Y 坐标渲染元素（用于 Page 内的绝对定位）
    fn render_element_at_y_absolute(&mut self, ui: &mut Ui, element: &LayoutElement, y: f32, parent_rect: egui::Rect, result: &mut RenderResult) -> f32 {
        match &element.element_type {
            ElementType::VBox => {
                // VBox: 递归渲染子元素
                let mut current_y = y;
                for child in &element.children {
                    current_y = self.render_element_at_y_absolute(ui, child, current_y, parent_rect, result);
                }
                current_y
            }
            ElementType::Spacer => {
                // Spacer: 只增加 Y 坐标，不渲染任何东西
                let height = element.attributes.height.unwrap_or(0.0);
                // 移除频繁的日志输出
                y + height
            }
            ElementType::HBox | ElementType::Image | ElementType::Button | ElementType::Checkbox | ElementType::Label => {
                // 获取元素高度
                let height = element.attributes.height.unwrap_or(40.0);

                // 创建一个从当前 Y 位置开始的矩形
                let element_rect = egui::Rect::from_min_size(
                    egui::pos2(parent_rect.min.x, y),
                    egui::vec2(parent_rect.width(), height)
                );

                // 创建子 UI，直接使用 painter 在指定矩形内绘制
                let mut child_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(element_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center))
                );

                self.render_element(&mut child_ui, element, result);

                y + height
            }
            _ => {
                // 其他元素使用默认渲染
                self.render_element(ui, element, result);
                y
            }
        }
    }

    /// 在指定 Y 坐标渲染元素，返回下一个元素的 Y 坐标（相对坐标版本，已废弃）
    fn render_element_at_y(&mut self, ui: &mut Ui, element: &LayoutElement, y: f32, result: &mut RenderResult) -> f32 {
        match &element.element_type {
            ElementType::VBox => {
                // VBox: 递归渲染子元素
                let mut current_y = y;
                for child in &element.children {
                    current_y = self.render_element_at_y(ui, child, current_y, result);
                }
                current_y
            }
            ElementType::Spacer => {
                // Spacer: 只增加 Y 坐标
                let height = element.attributes.height.unwrap_or(0.0);
                y + height
            }
            ElementType::HBox | ElementType::Image | ElementType::Button | ElementType::Checkbox | ElementType::Label => {
                // 获取元素高度
                let height = element.attributes.height.unwrap_or(40.0);

                // 在当前 Y 位置渲染元素
                let rect = egui::Rect::from_min_size(
                    egui::pos2(0.0, y),
                    egui::vec2(ui.available_width(), height)
                );

                let mut child_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center))
                );

                self.render_element(&mut child_ui, element, result);

                y + height
            }
            _ => {
                // 其他元素使用默认渲染
                self.render_element(ui, element, result);
                y
            }
        }
    }

    /// 渲染垂直布局
    fn render_vbox(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        let spacing = element.attributes.spacing.unwrap_or(0.0);
        let padding = element.attributes.padding;
        let align = element.attributes.align.as_deref().unwrap_or("top");
        let halign = element.attributes.get_custom("halign").map(|s| s.as_str()).unwrap_or("left");

        let available_height = ui.available_height();
        // 移除频繁的日志输出

        // 检查是否有 flex 子元素
        let has_flex_children = element.children.iter()
            .any(|child| child.attributes.flex.is_some());

        if has_flex_children {
            self.render_vbox_with_flex(ui, element, result);
        } else if align != "top" || halign != "left" {
            self.render_vbox_with_align(ui, element, result, align, halign);
        } else {
            // 使用 allocate_ui_with_layout 确保占据全部可用高度
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), available_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, spacing);

                    if let Some((top, _right, _bottom, _left)) = padding {
                        ui.add_space(top);
                    }

                    for (idx, child) in element.children.iter().enumerate() {
                        // 移除频繁的日志输出
                        self.render_element(ui, child, result);
                    }

                    if let Some((_top, _right, bottom, _left)) = padding {
                        ui.add_space(bottom);
                    }
                }
            );
        }
    }
    
    /// 渲染带对齐的垂直布局
    fn render_vbox_with_align(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult, align: &str, halign: &str) {
        let spacing = element.attributes.spacing.unwrap_or(0.0);
        let padding = element.attributes.padding;
        
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, spacing);
            
            if let Some((top, _right, _bottom, _left)) = padding {
                ui.add_space(top);
            }
            
            // 计算子元素总高度
            let mut total_height = 0.0;
            for child in &element.children {
                if let Some(height) = child.attributes.height {
                    total_height += height;
                }
            }
            total_height += spacing * (element.children.len() as f32 - 1.0).max(0.0);
            
            let available_height = ui.available_height();
            let remaining_height = (available_height - total_height).max(0.0);
            
            // 垂直对齐
            match align {
                "center" | "middle" => {
                    ui.add_space(remaining_height / 2.0);
                }
                "bottom" => {
                    ui.add_space(remaining_height);
                }
                "space-between" => {
                    let gap = if element.children.len() > 1 {
                        remaining_height / (element.children.len() as f32 - 1.0)
                    } else {
                        0.0
                    };
                    
                    for (i, child) in element.children.iter().enumerate() {
                        self.render_element_with_halign(ui, child, result, halign);
                        if i < element.children.len() - 1 {
                            ui.add_space(gap);
                        }
                    }
                    
                    if let Some((_top, _right, bottom, _left)) = padding {
                        ui.add_space(bottom);
                    }
                    return;
                }
                _ => {} // "top" - 默认，不添加空间
            }
            
            // 渲染子元素（带水平对齐）
            for child in &element.children {
                self.render_element_with_halign(ui, child, result, halign);
            }
            
            if let Some((_top, _right, bottom, _left)) = padding {
                ui.add_space(bottom);
            }
        });
    }
    
    /// 渲染元素（带水平对齐）
    fn render_element_with_halign(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult, halign: &str) {
        match halign {
            "center" => {
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 50.0); // 简单居中，TODO: 精确计算
                    self.render_element(ui, element, result);
                });
            }
            "right" => {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    self.render_element(ui, element, result);
                });
            }
            _ => {
                self.render_element(ui, element, result);
            }
        }
    }
    
    /// 渲染带 flex 的垂直布局
    fn render_vbox_with_flex(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        let spacing = element.attributes.spacing.unwrap_or(0.0);
        
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, spacing);
            
            // 第一遍：计算固定高度元素和总 flex 权重
            let mut fixed_height = 0.0;
            let mut total_flex = 0.0;
            
            for child in &element.children {
                if let Some(flex) = child.attributes.flex {
                    total_flex += flex;
                } else if let Some(height) = child.attributes.height {
                    fixed_height += height;
                }
            }
            
            // 计算剩余可用空间
            let available_height = ui.available_height();
            let spacing_total = spacing * (element.children.len() as f32 - 1.0).max(0.0);
            let remaining_height = (available_height - fixed_height - spacing_total).max(0.0);
            
            // 第二遍：渲染元素
            for child in &element.children {
                if let Some(flex) = child.attributes.flex {
                    // Flex 元素：分配剩余空间
                    let flex_height = if total_flex > 0.0 {
                        remaining_height * (flex / total_flex)
                    } else {
                        0.0
                    };
                    
                    if child.element_type == crate::layout::element::ElementType::Spacer {
                        // Spacer 占用空间但不渲染内容
                        ui.add_space(flex_height);
                    } else {
                        // 其他 flex 元素在分配的高度内渲染
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), flex_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                self.render_element(ui, child, result);
                            },
                        );
                    }
                } else {
                    // 非 flex 元素：正常渲染
                    self.render_element(ui, child, result);
                }
            }
        });
    }

    /// 渲染水平布局
    fn render_hbox(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        let spacing = element.attributes.spacing.unwrap_or(0.0);
        let padding = element.attributes.padding;
        let align = element.attributes.align.as_deref().unwrap_or("left");
        let valign = element.attributes.get_custom("valign").map(|s| s.as_str()).unwrap_or("top");
        let height = element.attributes.height;

        // 检查是否有 flex 子元素
        let has_flex_children = element.children.iter()
            .any(|child| child.attributes.flex.is_some());

        // 如果指定了高度，先分配固定高度的区域
        if let Some(h) = height {
            let response = ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), h),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(spacing, 0.0);

                    if let Some((_top, _right, _bottom, left)) = padding {
                        ui.add_space(left);
                    }

                    if has_flex_children {
                        self.render_hbox_flex_children(ui, element, result);
                    } else {
                        for child in &element.children {
                            self.render_element(ui, child, result);
                        }
                    }

                    if let Some((_top, right, _bottom, _left)) = padding {
                        ui.add_space(right);
                    }
                }
            );
            // 移除频繁的日志输出
        } else if has_flex_children {
            self.render_hbox_with_flex(ui, element, result);
        } else if align != "left" || valign != "top" {
            self.render_hbox_with_align(ui, element, result, align, valign);
        } else {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(spacing, 0.0);

                if let Some((_top, _right, _bottom, left)) = padding {
                    ui.add_space(left);
                }

                for child in &element.children {
                    self.render_element(ui, child, result);
                }

                if let Some((_top, right, _bottom, _left)) = padding {
                    ui.add_space(right);
                }
            });
        }
    }

    /// 渲染 HBox 的 flex 子元素（不包含外层 horizontal）
    fn render_hbox_flex_children(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        let spacing = element.attributes.spacing.unwrap_or(0.0);

        // 第一遍：计算固定宽度元素和总 flex 权重
        let mut fixed_width = 0.0;
        let mut total_flex = 0.0;

        for child in &element.children {
            if let Some(flex) = child.attributes.flex {
                total_flex += flex;
            } else if let Some(width) = child.attributes.width {
                fixed_width += width;
            }
        }

        // 计算剩余可用空间
        let available_width = ui.available_width();
        let spacing_total = spacing * (element.children.len() as f32 - 1.0).max(0.0);
        let remaining_width = (available_width - fixed_width - spacing_total).max(0.0);

        // 第二遍：渲染元素
        for child in &element.children {
            if let Some(flex) = child.attributes.flex {
                // Flex 元素：分配剩余空间
                let flex_width = if total_flex > 0.0 {
                    remaining_width * (flex / total_flex)
                } else {
                    0.0
                };

                if child.element_type == crate::layout::element::ElementType::Spacer {
                    // Spacer 占用空间但不渲染内容
                    ui.add_space(flex_width);
                } else {
                    // 其他 flex 元素在分配的宽度内渲染
                    ui.allocate_ui_with_layout(
                        egui::vec2(flex_width, ui.available_height()),
                        egui::Layout::left_to_right(egui::Align::Min),
                        |ui| {
                            self.render_element(ui, child, result);
                        },
                    );
                }
            } else {
                // 非 flex 元素：正常渲染
                self.render_element(ui, child, result);
            }
        }
    }
    
    /// 渲染带对齐的水平布局
    fn render_hbox_with_align(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult, align: &str, valign: &str) {
        let spacing = element.attributes.spacing.unwrap_or(0.0);
        let padding = element.attributes.padding;
        
        // 确定垂直对齐方式
        let vertical_align = match valign {
            "center" | "middle" => egui::Align::Center,
            "bottom" => egui::Align::Max,
            _ => egui::Align::Min,
        };
        
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(spacing, 0.0);
            
            if let Some((_top, _right, _bottom, left)) = padding {
                ui.add_space(left);
            }
            
            // 计算子元素总宽度
            let mut total_width = 0.0;
            for child in &element.children {
                if let Some(width) = child.attributes.width {
                    total_width += width;
                } else if let Some(min_width) = child.attributes.min_width {
                    total_width += min_width;
                }
            }
            total_width += spacing * (element.children.len() as f32 - 1.0).max(0.0);
            
            let available_width = ui.available_width();
            let remaining_width = (available_width - total_width).max(0.0);
            
            // 水平对齐
            match align {
                "center" => {
                    ui.add_space(remaining_width / 2.0);
                }
                "right" => {
                    ui.add_space(remaining_width);
                }
                "space-between" => {
                    let gap = if element.children.len() > 1 {
                        remaining_width / (element.children.len() as f32 - 1.0)
                    } else {
                        0.0
                    };
                    
                    for (i, child) in element.children.iter().enumerate() {
                        self.render_element_with_valign(ui, child, result, vertical_align);
                        if i < element.children.len() - 1 {
                            ui.add_space(gap);
                        }
                    }
                    
                    if let Some((_top, right, _bottom, _left)) = padding {
                        ui.add_space(right);
                    }
                    return;
                }
                _ => {} // "left" - 默认，不添加空间
            }
            
            // 渲染子元素（带垂直对齐）
            for child in &element.children {
                self.render_element_with_valign(ui, child, result, vertical_align);
            }
            
            if let Some((_top, right, _bottom, _left)) = padding {
                ui.add_space(right);
            }
        });
    }
    
    /// 渲染元素（带垂直对齐）
    fn render_element_with_valign(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult, valign: egui::Align) {
        ui.with_layout(egui::Layout::top_down(valign), |ui| {
            self.render_element(ui, element, result);
        });
    }
    
    /// 渲染带 flex 的水平布局
    fn render_hbox_with_flex(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        let spacing = element.attributes.spacing.unwrap_or(0.0);
        
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(spacing, 0.0);
            
            // 第一遍：计算固定宽度元素和总 flex 权重
            let mut fixed_width = 0.0;
            let mut total_flex = 0.0;
            
            for child in &element.children {
                if let Some(flex) = child.attributes.flex {
                    total_flex += flex;
                } else if let Some(width) = child.attributes.width {
                    fixed_width += width;
                }
            }
            
            // 计算剩余可用空间
            let available_width = ui.available_width();
            let spacing_total = spacing * (element.children.len() as f32 - 1.0).max(0.0);
            let remaining_width = (available_width - fixed_width - spacing_total).max(0.0);
            
            // 第二遍：渲染元素
            for child in &element.children {
                if let Some(flex) = child.attributes.flex {
                    // Flex 元素：分配剩余空间
                    let flex_width = if total_flex > 0.0 {
                        remaining_width * (flex / total_flex)
                    } else {
                        0.0
                    };
                    
                    if child.element_type == crate::layout::element::ElementType::Spacer {
                        // Spacer 占用空间但不渲染内容
                        ui.add_space(flex_width);
                    } else {
                        // 其他 flex 元素在分配的宽度内渲染
                        ui.allocate_ui_with_layout(
                            egui::vec2(flex_width, ui.available_height()),
                            egui::Layout::left_to_right(egui::Align::Min),
                            |ui| {
                                self.render_element(ui, child, result);
                            },
                        );
                    }
                } else {
                    // 非 flex 元素：正常渲染
                    self.render_element(ui, child, result);
                }
            }
        });
    }

    /// 渲染空白占位
    fn render_spacer(&mut self, ui: &mut Ui, element: &LayoutElement, _result: &mut RenderResult) {
        let width = element.attributes.width.unwrap_or(0.0);
        let height = element.attributes.height.unwrap_or(0.0);

        // 水平方向：使用 width 占位
        // 垂直方向：使用 height 占位
        // 空 Spacer：作为弹性空间（在 HBox 中填充剩余空间）
        if width > 0.0 {
            // 水平 Spacer（在 HBox 中）- 使用可用高度确保占位正确
            let spacer_height = ui.available_height().max(0.0);
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(width, spacer_height),
                egui::Sense::hover()
            );
            // 移除频繁的日志输出
        } else if height > 0.0 {
            // 垂直 Spacer（在 VBox 中）- 使用 allocate_exact_size 而不是 add_space
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), height),
                egui::Sense::hover()
            );
            // 移除频繁的日志输出
        } else {
            // 空 Spacer：弹性空间（填充所有剩余空间）
            ui.allocate_space(egui::vec2(ui.available_width(), 0.0));
            // 移除频繁的日志输出
        }
    }

    /// 渲染弹性空间
    fn render_flex(&mut self, ui: &mut Ui, _element: &LayoutElement, _result: &mut RenderResult) {
        ui.allocate_ui_with_layout(
            ui.available_size(),
            egui::Layout::left_to_right(Align::LEFT),
            |ui| {
                ui.allocate_ui(ui.available_size(), |_ui| {});
            }
        );
    }

    /// 渲染按钮
    fn render_button(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        let id = element.attributes.id.as_ref().unwrap_or(&"".to_string()).clone();
        let text = self.get_display_text(&element.attributes);
        let style_type = StyleType::from(&element.attributes);
        let enabled = element.attributes.enabled.unwrap_or(true);

        let width = element.attributes.width.unwrap_or(120.0);
        let height = element.attributes.height.unwrap_or(40.0);

        // 获取字体和颜色配置
        let font_size = element.attributes.get_custom("font_size")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(14.0);

        let text_color = element.attributes.color.as_ref()
            .and_then(|c| self.parse_color(c))
            .unwrap_or(Color32::WHITE);

        // 检查是否有绝对定位（position 属性）
        let (rect, response) = if let Some(position_str) = element.attributes.get_custom("position") {
            // 解析 position="x,y"
            let coords: Vec<f32> = position_str.split(',')
                .map(|s| s.trim().parse::<f32>().ok())
                .filter_map(|x| x)
                .collect();
            
            if coords.len() == 2 {
                // 绝对定位：在指定位置分配空间
                let pos = egui::pos2(coords[0], coords[1]);
                let size = egui::vec2(width, height);
                let rect = egui::Rect::from_min_size(pos, size);
                let response = ui.allocate_rect(rect, egui::Sense::click());
                (rect, response)
            } else {
                // 解析失败，使用默认布局
                ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click())
            }
        } else {
            // 默认布局流
            ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click())
        };

        // 绘制按钮背景和文本
        if ui.is_rect_visible(rect) {
            // 从 custom 属性读取图片配置
            let normalimage = element.attributes.get_custom("normalimage");
            let hotimage = element.attributes.get_custom("hotimage");
            let pushedimage = element.attributes.get_custom("pushedimage");
            let disabledimage = element.attributes.get_custom("disabledimage");

            // 调试日志已移除，避免每帧都输出（egui 的 update 函数会被频繁调用）
            // 如果需要调试，可以使用 tracing::debug! 并设置日志级别

            // 根据按钮状态选择背景图片
            let bg_image = if !enabled {
                disabledimage.or(normalimage)
            } else if response.is_pointer_button_down_on() {
                pushedimage.or(hotimage).or(normalimage)
            } else if response.hovered() {
                hotimage.or(normalimage)
            } else {
                normalimage
            };

            // 如果有背景图片，渲染图片背景
            if let Some(img_path_str) = bg_image {
                let image_path = Self::parse_image_path(img_path_str);
                if let Some(texture) = self.resource_cache.get_background(ui.ctx(), &self.dpi_config, &image_path.path) {
                    // 如果指定了 dest 裁剪区域，计算 UV 坐标
                    let uv_rect = if let Some((x1, y1, x2, y2)) = image_path.dest {
                        // 获取纹理的实际尺寸
                        let tex_size = texture.size();
                        // 计算 UV 坐标（归一化到 0-1）
                        let uv_min_x = x1 / tex_size[0] as f32;
                        let uv_min_y = y1 / tex_size[1] as f32;
                        let uv_max_x = x2 / tex_size[0] as f32;
                        let uv_max_y = y2 / tex_size[1] as f32;
                        egui::Rect::from_min_max(
                            egui::pos2(uv_min_x, uv_min_y),
                            egui::pos2(uv_max_x, uv_max_y)
                        )
                    } else {
                        // 使用整个纹理
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))
                    };
                    
                    ui.painter().image(
                        texture.id(),
                        rect,
                        uv_rect,
                        egui::Color32::WHITE,
                    );
                } else {
                    // 图片加载失败，使用纯色背景
                    ui.painter().rect_filled(
                        rect,
                        egui::CornerRadius::same(4),
                        if enabled {
                            if response.hovered() {
                                Color32::from_rgb(70, 70, 70)
                            } else {
                                Color32::from_rgb(50, 50, 50)
                            }
                        } else {
                            Color32::from_rgb(30, 30, 30)
                        }
                    );
                }
            } else {
                // 没有配置图片：不绘制背景（文本按钮）
            }

            // 绘制按钮文本
            let text_pos = rect.center();
            ui.painter().text(
                text_pos,
                egui::Align2::CENTER_CENTER,
                &text,
                egui::FontId::proportional(font_size),
                text_color
            );
        }

        // 移除频繁的日志输出

        if response.clicked() && enabled {
            result.button_clicks.insert(id.clone(), true);
        }

        // 记录按钮响应
        if let Some(id) = element.attributes.id.as_ref() {
            result.button_responses.insert(id.clone(), response);
        }
    }

    /// 渲染标签
    fn render_label(&mut self, ui: &mut Ui, element: &LayoutElement, _result: &mut RenderResult) {
        let text = self.get_display_text(&element.attributes);
        let _style_type = StyleType::from(&element.attributes);
        let _label_style = self.style_engine.get_label_style(&_style_type);

        // 获取字体大小
        let font_size = element.attributes.get_custom("font_size")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(12.0);

        // 获取文字颜色
        let text_color = element.attributes.color.as_ref()
            .and_then(|c| self.parse_color(c))
            .unwrap_or(Color32::WHITE);

        // 获取尺寸约束
        let width = element.attributes.width;
        let height = element.attributes.height;

        // 如果指定了宽度和高度，使用精确尺寸
        if let (Some(w), Some(h)) = (width, height) {
            let (rect, _response) = ui.allocate_exact_size(
                egui::vec2(w, h),
                egui::Sense::hover()
            );

            if ui.is_rect_visible(rect) {
                // 在分配的区域内绘制文本（居中）
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &text,
                    egui::FontId::proportional(font_size),
                    text_color
                );
            }
        } else if let Some(w) = width {
            // 只指定了宽度
            ui.allocate_ui_with_layout(
                egui::vec2(w, 0.0),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    let mut rich_text = egui::RichText::new(&text)
                        .size(font_size)
                        .color(text_color);

                    let label = egui::Label::new(rich_text);
                    ui.add(label);
                },
            );
        } else {
            // 没有尺寸约束，使用默认渲染
            let rich_text = egui::RichText::new(&text)
                .size(font_size)
                .color(text_color);

            let label = egui::Label::new(rich_text);
            ui.add(label);
        }
    }
    
    /// 解析颜色字符串
    fn parse_color(&self, color_str: &str) -> Option<egui::Color32> {
        // 支持 #RRGGBB 格式
        if color_str.starts_with('#') && color_str.len() == 7 {
            let r = u8::from_str_radix(&color_str[1..3], 16).ok()?;
            let g = u8::from_str_radix(&color_str[3..5], 16).ok()?;
            let b = u8::from_str_radix(&color_str[5..7], 16).ok()?;
            return Some(egui::Color32::from_rgb(r, g, b));
        }
        None
    }

    /// 渲染复选框
    fn render_checkbox(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        let id = element.attributes.id.as_ref().unwrap_or(&"".to_string()).clone();
        let text = self.get_display_text(&element.attributes);
        let _style_type = StyleType::from(&element.attributes);
        let _enabled = element.attributes.enabled.unwrap_or(true);

        // 解析文本，检查是否包含链接
        let segments = self.parse_text_with_links(&text);

        // 如果文本中包含链接，使用自定义渲染
        if segments.iter().any(|s| matches!(s, TextSegment::Link { .. })) {
            self.render_checkbox_with_links(ui, element, &segments, result);
        } else {
            // 标准渲染（无链接）
            self.render_checkbox_simple(ui, element, &text, result);
        }
    }

    /// 渲染简单复选框（无内联链接）
    fn render_checkbox_simple(&mut self, ui: &mut Ui, element: &LayoutElement, text: &str, result: &mut RenderResult) {
        let id = element.attributes.id.as_ref().unwrap_or(&"".to_string()).clone();

        // 获取字体和颜色配置
        let font_size = element.attributes.get_custom("font_size")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(12.0);

        let text_color = element.attributes.color.as_ref()
            .and_then(|c| self.parse_color(c))
            .unwrap_or(Color32::WHITE);

        let enabled = element.attributes.enabled.unwrap_or(true);

        // 获取尺寸约束
        let width = element.attributes.width;
        let height = element.attributes.height;

        // 获取当前状态
        let current_checked = self.interaction_state.checkbox_states.get(&id).copied().unwrap_or(false);
        let mut checkbox_state = self.interaction_state.checkbox_states.entry(id.clone()).or_insert(current_checked).clone();

        // 如果指定了宽度和高度，使用精确尺寸
        let response = if let (Some(w), Some(h)) = (width, height) {
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(w, h),
                egui::Sense::click()
            );

            if ui.is_rect_visible(rect) {
                // 从 custom 属性读取图片配置
                let normalimage = element.attributes.get_custom("normalimage");
                let selectedimage = element.attributes.get_custom("selectedimage");
                let disabledimage = element.attributes.get_custom("disabledimage");

                // 从 custom 属性读取 textpadding（格式：left,top,right,bottom）
                let text_padding_left = element.attributes.get_custom("textpadding")
                    .and_then(|s| s.split(',').next())
                    .and_then(|s| s.trim().parse::<f32>().ok())
                    .unwrap_or(20.0);

                // 使用图片渲染复选框
                // 根据 DPI 使用不同的复选框大小：1x = 16px, 2x = 32px
                let checkbox_size = if self.dpi_config.use_2x { 32.0 } else { 16.0 };
                let checkbox_rect = egui::Rect::from_min_size(
                    rect.min,
                    egui::Vec2::splat(checkbox_size)
                );

                // 根据状态选择图片
                let checkbox_image = if !enabled {
                    disabledimage.or(normalimage)
                } else if checkbox_state {
                    selectedimage.or(normalimage)
                } else {
                    normalimage
                };

                // 渲染复选框图片
                if let Some(img_path_str) = checkbox_image {
                    let image_path = Self::parse_image_path(img_path_str);
                    if let Some(texture) = self.resource_cache.get_background(ui.ctx(), &self.dpi_config, &image_path.path) {
                        // 如果指定了 dest 裁剪区域，计算 UV 坐标
                        let uv_rect = if let Some((x1, y1, x2, y2)) = image_path.dest {
                            // 获取纹理的实际尺寸
                            let tex_size = texture.size();
                            // 计算 UV 坐标（归一化到 0-1）
                            let uv_min_x = x1 / tex_size[0] as f32;
                            let uv_min_y = y1 / tex_size[1] as f32;
                            let uv_max_x = x2 / tex_size[0] as f32;
                            let uv_max_y = y2 / tex_size[1] as f32;
                            egui::Rect::from_min_max(
                                egui::pos2(uv_min_x, uv_min_y),
                                egui::pos2(uv_max_x, uv_max_y)
                            )
                        } else {
                            // 使用整个纹理
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))
                        };
                        
                        ui.painter().image(
                            texture.id(),
                            checkbox_rect,
                            uv_rect,
                            egui::Color32::WHITE,
                        );
                    } else {
                        // 图片加载失败，使用简单方框
                        ui.painter().rect_stroke(
                            checkbox_rect,
                            egui::CornerRadius::same(2),
                            egui::Stroke::new(1.0, Color32::GRAY),
                            egui::epaint::StrokeKind::Outside
                        );

                        if checkbox_state {
                            ui.painter().rect_filled(
                                checkbox_rect.shrink(3.0),
                                egui::CornerRadius::same(1),
                                Color32::WHITE
                            );
                        }
                    }
                } else {
                    // 没有配置图片，使用简单方框
                    ui.painter().rect_stroke(
                        checkbox_rect,
                        egui::CornerRadius::same(2),
                        egui::Stroke::new(1.0, Color32::GRAY),
                        egui::epaint::StrokeKind::Outside
                    );

                    if checkbox_state {
                        ui.painter().rect_filled(
                            checkbox_rect.shrink(3.0),
                            egui::CornerRadius::same(1),
                            Color32::WHITE
                        );
                    }
                }

                // 绘制文本（在复选框右侧，使用 textpadding）
                let text_pos = egui::pos2(rect.min.x + text_padding_left, rect.center().y);
                ui.painter().text(
                    text_pos,
                    egui::Align2::LEFT_CENTER,
                    text,
                    egui::FontId::proportional(font_size),
                    text_color
                );
            }

            response
        } else {
            // 没有尺寸约束，使用默认 checkbox
            let checkbox = egui::Checkbox::new(
                &mut checkbox_state,
                egui::RichText::new(text)
                    .size(font_size)
                    .color(text_color)
            );

            ui.add(checkbox)
        };

        // 移除频繁的日志输出

        if response.clicked() {
            let new_checked = !current_checked;
            self.interaction_state.checkbox_states.insert(id.clone(), new_checked);
            result.checkbox_changes.insert(id.clone(), new_checked);
        }

        // 记录复选框响应
        if let Some(id) = element.attributes.id.as_ref() {
            result.checkbox_responses.insert(id.clone(), response);
        }
    }

    /// 渲染带内联链接的复选框
    fn render_checkbox_with_links(&mut self, ui: &mut Ui, element: &LayoutElement, segments: &[TextSegment], result: &mut RenderResult) {
        let id = element.attributes.id.as_ref().unwrap_or(&"".to_string()).clone();

        // 获取字体大小和颜色配置
        let font_size = element.attributes.get_custom("font_size")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(12.0);

        let text_color = element.attributes.color.as_ref()
            .and_then(|c| self.parse_color(c))
            .unwrap_or(Color32::WHITE);

        let link_color = Color32::from_rgb(0, 255, 232); // #00FFE8

        // 获取当前状态
        let mut current_checked = self.interaction_state.checkbox_states.get(&id).copied().unwrap_or(false);

        // 使用水平布局渲染 checkbox 和文本
        let changed = ui.horizontal(|ui| {
            // 渲染 checkbox 框
            let checkbox = egui::Checkbox::without_text(&mut current_checked);
            let checkbox_response = ui.add(checkbox);

            // 记录复选框响应
            if let Some(elem_id) = element.attributes.id.as_ref() {
                result.checkbox_responses.insert(elem_id.clone(), checkbox_response.clone());
            }

            // 渲染文本片段（包含可点击链接）
            for segment in segments {
                match segment {
                    TextSegment::Text(text) => {
                        let label = egui::Label::new(
                            egui::RichText::new(text)
                                .size(font_size)
                                .color(text_color)
                        );
                        ui.add(label);
                    }
                    TextSegment::Link { id: link_id, text: link_text } => {
                        let link_rich_text = egui::RichText::new(link_text)
                            .size(font_size)
                            .color(link_color);

                        let button = egui::Button::new(link_rich_text)
                            .frame(false)  // 无边框，像普通链接
                            .fill(Color32::TRANSPARENT);

                        let link_response = ui.add(button);

                        // 悬停效果
                        if link_response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }

                        // 点击事件
                        if link_response.clicked() {
                            result.link_clicks.insert(link_id.clone(), true);
                            tracing::info!("Link clicked: {}", link_id); // 保留，因为这是用户交互事件
                        }
                    }
                }
            }

            checkbox_response.changed()
        }).inner;

        // 更新状态
        if changed {
            self.interaction_state.checkbox_states.insert(id.clone(), current_checked);
            result.checkbox_changes.insert(id.clone(), current_checked);
        }
    }

    /// 渲染文本输入框
    fn render_text_input(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        let id = element.attributes.id.as_ref().unwrap_or(&"".to_string()).clone();
        let _style_type = StyleType::from(&element.attributes);
        let _enabled = element.attributes.enabled.unwrap_or(true);
        
        // 获取当前内容
        let _current_text = self.interaction_state.text_inputs.get(&id).cloned().unwrap_or_default();
        
        let mut text_input = egui::TextEdit::singleline(self.interaction_state.text_inputs.entry(id.clone()).or_insert_with(String::new));
        
        if let Some(width) = element.attributes.width {
            text_input = text_input.desired_width(width);
        }
        if let Some(height) = element.attributes.height {
            text_input = text_input.desired_rows((height / 20.0) as usize);
        }
        
        let response = ui.add(text_input);
        
        if response.changed() {
            result.text_input_changes.insert(id.clone(), self.interaction_state.text_inputs.get(&id).cloned().unwrap_or_default());
        }
        
        // 记录文本输入响应
        if let Some(id) = element.attributes.id.as_ref() {
            result.text_input_responses.insert(id.clone(), response);
        }
    }

    /// 渲染图片
    fn render_image(&mut self, ui: &mut Ui, element: &LayoutElement, _result: &mut RenderResult) {
        if let Some(icon) = &element.attributes.icon {
            if let Some(texture) = self.resource_cache.get_background(ui.ctx(), &self.dpi_config, icon) {
                // 如果 XML 中指定了尺寸，使用指定的尺寸
                // 否则使用纹理的实际渲染尺寸（会自动处理 2x 资源）
                let size = if let (Some(w), Some(h)) = (element.attributes.width, element.attributes.height) {
                    egui::Vec2::new(w, h)
                } else {
                    self.dpi_config.get_render_size(texture)
                };

                // 分配精确尺寸以确保图片不会被 egui 调整大小
                let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());

                // 在分配的区域内绘制图片
                if ui.is_rect_visible(rect) {
                    ui.painter().image(
                        texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }

                // 移除频繁的日志输出
            }
        }
    }

    /// 渲染进度条
    fn render_progress_bar(&mut self, ui: &mut Ui, element: &LayoutElement, _result: &mut RenderResult) {
        let progress = element.attributes.progress.unwrap_or(0.0);
        let _style_type = StyleType::from(&element.attributes);
        let _progress_style = self.style_engine.get_progress_style(&_style_type);

        let width = element.attributes.width.unwrap_or(ui.available_width());
        let height = element.attributes.height.unwrap_or(6.0);

        // 分配进度条区域
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(width, height),
            egui::Sense::hover()
        );

        if ui.is_rect_visible(rect) {
            // 先绘制背景图片（未填充部分）
            let bg_image = "assets/progress_bg.png";
            if let Some(bg_texture) = self.resource_cache.get_background(ui.ctx(), &self.dpi_config, bg_image) {
                ui.painter().image(
                    bg_texture.id(),
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            } else {
                // 背景图片加载失败，绘制灰色背景
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::same(3),
                    Color32::from_rgb(40, 40, 40)
                );
            }

            // 绘制前景图片（填充部分）
            let fg_image = "assets/progress_fg.png";
            let progress_width = width * progress;
            if progress_width > 0.0 {
                let progress_rect = Rect::from_min_size(
                    rect.min,
                    Vec2::new(progress_width, height)
                );

                if let Some(fg_texture) = self.resource_cache.get_background(ui.ctx(), &self.dpi_config, fg_image) {
                    // 根据进度裁剪前景图片
                    ui.painter().image(
                        fg_texture.id(),
                        progress_rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(progress, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    // 前景图片加载失败，使用纯色填充
                    ui.painter().rect_filled(
                        progress_rect,
                        egui::CornerRadius::same(3),
                        Color32::from_rgb(0, 196, 178)
                    );
                }
            }
        }
    }

    /// 渲染分隔线
    fn render_divider(&mut self, ui: &mut Ui, element: &LayoutElement, _result: &mut RenderResult) {
        let style_type = StyleType::from(&element.attributes);
        let divider_style = self.style_engine.get_divider_style(&style_type);
        
        let width = element.attributes.width.unwrap_or(ui.available_width());
        let height = element.attributes.height.unwrap_or(1.0);
        
        ui.add_space(5.0);
        ui.painter().rect_filled(
            Rect::from_min_size(ui.cursor().min, Vec2::new(width, height)),
            CornerRadius::same(0),
            divider_style.color,
        );
        ui.add_space(5.0);
    }

    /// 渲染浮动层
    fn render_overlay(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        if let Some((x, y)) = element.attributes.position {
            let width = element.attributes.width.unwrap_or(200.0);
            let height = element.attributes.height.unwrap_or(100.0);
            
            egui::Area::new(egui::Id::new(element.attributes.id.as_ref().unwrap_or(&"overlay".to_string())))
                .fixed_pos(Pos2::new(x, y))
                .show(ui.ctx(), |ui| {
                    ui.allocate_ui(Vec2::new(width, height), |ui| {
                        for child in &element.children {
                            self.render_element(ui, child, result);
                        }
                    });
                });
        }
    }

    /// 获取显示文本（处理国际化）
    fn get_display_text(&self, attrs: &ElementAttributes) -> String {
        // 优先使用 text_i18n
        if let Some(i18n_key) = &attrs.text_i18n {
            if let Some(translated) = self.i18n_strings.get(i18n_key) {
                return translated.clone();
            }
        }
        
        // 其次使用 text（支持 @ 前缀）
        if let Some(text) = &attrs.text {
            if text.starts_with('@') {
                // 国际化字符串
                let key = &text[1..];
                self.i18n_strings.get(key).cloned().unwrap_or_else(|| text.clone())
            } else {
                text.clone()
            }
        } else {
            String::new()
        }
    }

    /// 获取按钮点击状态
    pub fn get_button_clicked(&self, id: &str) -> bool {
        self.interaction_state.button_clicks.get(id).copied().unwrap_or(false)
    }

    /// 获取复选框状态
    pub fn get_checkbox_checked(&self, id: &str) -> bool {
        self.interaction_state.checkbox_states.get(id).copied().unwrap_or(false)
    }

    /// 获取文本输入内容
    pub fn get_text_input_value(&self, id: &str) -> String {
        self.interaction_state.text_inputs.get(id).cloned().unwrap_or_default()
    }

    /// 设置文本输入内容
    pub fn set_text_input_value(&mut self, id: &str, value: String) {
        self.interaction_state.text_inputs.insert(id.to_string(), value);
    }

    /// 清除交互状态
    pub fn clear_interaction_state(&mut self) {
        self.interaction_state = InteractionState::default();
    }

    /// 解析带链接标记的文本
    /// 支持 Markdown 格式: "普通文本[链接文本](链接ID)更多文本"
    fn parse_text_with_links(&self, text: &str) -> Vec<TextSegment> {
        let mut segments = Vec::new();
        let mut current_text = String::new();
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '[' {
                // 读取链接文本 (直到 ']')
                let mut link_text = String::new();
                let mut found_closing_bracket = false;

                while let Some(c) = chars.next() {
                    if c == ']' {
                        found_closing_bracket = true;
                        break;
                    }
                    link_text.push(c);
                }

                // 检查是否紧跟 '('
                if found_closing_bracket && chars.peek() == Some(&'(') {
                    chars.next(); // 跳过 '('

                    // 读取链接 ID (直到 ')')
                    let mut link_id = String::new();
                    let mut found_closing_paren = false;

                    while let Some(c) = chars.next() {
                        if c == ')' {
                            found_closing_paren = true;
                            break;
                        }
                        link_id.push(c);
                    }

                    if found_closing_paren && !link_text.is_empty() && !link_id.is_empty() {
                        // 保存当前文本
                        if !current_text.is_empty() {
                            segments.push(TextSegment::Text(current_text.clone()));
                            current_text.clear();
                        }

                        // 添加链接段
                        segments.push(TextSegment::Link {
                            id: link_id,
                            text: link_text,
                        });
                    } else {
                        // 解析失败，把原始文本加回去
                        current_text.push('[');
                        current_text.push_str(&link_text);
                        current_text.push(']');
                        current_text.push('(');
                        current_text.push_str(&link_id);
                        if found_closing_paren {
                            current_text.push(')');
                        }
                    }
                } else {
                    // 不是链接，只是普通的方括号
                    current_text.push('[');
                    current_text.push_str(&link_text);
                    if found_closing_bracket {
                        current_text.push(']');
                    }
                }
            } else {
                current_text.push(ch);
            }
        }

        // 保存剩余文本
        if !current_text.is_empty() {
            segments.push(TextSegment::Text(current_text));
        }

        segments
    }
}

/// 渲染结果
#[derive(Debug, Default)]
pub struct RenderResult {
    /// 按钮点击事件
    pub button_clicks: HashMap<String, bool>,
    /// 复选框状态变化
    pub checkbox_changes: HashMap<String, bool>,
    /// 文本输入变化
    pub text_input_changes: HashMap<String, String>,
    /// 按钮响应
    pub button_responses: HashMap<String, Response>,
    /// 复选框响应
    pub checkbox_responses: HashMap<String, Response>,
    /// 文本输入响应
    pub text_input_responses: HashMap<String, Response>,
    /// 链接点击事件（用于 checkbox 内联链接）
    pub link_clicks: HashMap<String, bool>,
}

/// 文本片段类型
#[derive(Debug, Clone)]
enum TextSegment {
    /// 普通文本
    Text(String),
    /// 可点击链接
    Link { id: String, text: String },
}

impl RenderResult {
    /// 创建新的渲染结果
    pub fn new() -> Self {
        Self::default()
    }

    /// 检查按钮是否被点击
    pub fn is_button_clicked(&self, id: &str) -> bool {
        self.button_clicks.get(id).copied().unwrap_or(false)
    }

    /// 检查复选框是否被改变
    pub fn is_checkbox_changed(&self, id: &str) -> bool {
        self.checkbox_changes.contains_key(id)
    }

    /// 获取复选框新状态
    pub fn get_checkbox_new_state(&self, id: &str) -> Option<bool> {
        self.checkbox_changes.get(id).copied()
    }

    /// 检查链接是否被点击
    pub fn is_link_clicked(&self, id: &str) -> bool {
        self.link_clicks.get(id).copied().unwrap_or(false)
    }

    /// 检查文本输入是否被改变
    pub fn is_text_input_changed(&self, id: &str) -> bool {
        self.text_input_changes.contains_key(id)
    }

    /// 获取文本输入新值
    pub fn get_text_input_new_value(&self, id: &str) -> Option<&String> {
        self.text_input_changes.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::InstallerConfig;

    #[test]
    fn test_layout_renderer_creation() {
        let config = InstallerConfig::default();
        let dpi_config = DpiConfig::new(&config);
        let i18n_strings = HashMap::new();
        
        let renderer = LayoutRenderer::new(dpi_config, i18n_strings);
        assert!(renderer.interaction_state.button_clicks.is_empty());
        assert!(renderer.interaction_state.checkbox_states.is_empty());
        assert!(renderer.interaction_state.text_inputs.is_empty());
    }

    #[test]
    fn test_render_result() {
        let mut result = RenderResult::new();
        assert!(!result.is_button_clicked("test"));
        assert!(!result.is_checkbox_changed("test"));
        assert!(!result.is_text_input_changed("test"));
        
        result.button_clicks.insert("test".to_string(), true);
        assert!(result.is_button_clicked("test"));
    }

    #[test]
    fn test_display_text_resolution() {
        let config = InstallerConfig::default();
        let dpi_config = DpiConfig::new(&config);
        let mut i18n_strings = HashMap::new();
        i18n_strings.insert("welcome.title".to_string(), "欢迎".to_string());
        
        let renderer = LayoutRenderer::new(dpi_config, i18n_strings);
        
        let attrs = ElementAttributes::new().with_text("@welcome.title");
        let text = renderer.get_display_text(&attrs);
        assert_eq!(text, "欢迎");
        
        let attrs = ElementAttributes::new().with_text("Normal text");
        let text = renderer.get_display_text(&attrs);
        assert_eq!(text, "Normal text");
    }
}
