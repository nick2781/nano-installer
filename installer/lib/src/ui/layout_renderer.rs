//! 布局渲染器
//!
//! 将布局树渲染到egui UI

use crate::layout::taffy_bridge::{ComputedLayout, TaffyBridge};
use crate::layout::{ElementAttributes, ElementType, LayoutElement, LayoutTree};
use crate::ui::{
    dpi_handler::DpiConfig,
    style_engine::{StyleEngine, StyleType},
};
use egui::{Align, Color32, CornerRadius, Pos2, Rect, Response, Ui, Vec2};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

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
    /// Taffy 布局引擎桥接
    taffy_bridge: TaffyBridge,
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
    /// 自绘下拉框展开状态
    select_open_states: HashMap<String, bool>,
    /// 悬停状态
    hover_states: HashMap<String, bool>,
}

/// 图片路径和裁剪信息
#[derive(Debug, Clone)]
struct ImagePath {
    path: String,
    dest: Option<(f32, f32, f32, f32)>, // (x1, y1, x2, y2) 裁剪区域
    corner: Option<(f32, f32, f32, f32)>, // (x1, y1, x2, y2) 圆角参数
    fade: Option<f32>,                  // 透明度 (0.0-1.0)
}

#[derive(Debug, Clone)]
enum InlineButtonGlyph {
    Image(ImagePath),
    ChevronDown,
    ChevronUp,
}

#[derive(Debug, Clone)]
struct InlineButtonIcon {
    glyph: InlineButtonGlyph,
    size: f32,
    gap: f32,
}

impl LayoutRenderer {
    /// 解析图片路径，支持 NSIS 格式：file='path' dest='x1,y1,x2,y2' corner='x1,y1,x2,y2' fade='value'
    fn parse_image_path(image_attr: &str) -> ImagePath {
        let mut path = String::new();
        let mut dest = None;
        let mut corner = None;
        let mut fade = None;

        // 解析 file='...'
        if let Some(start) = image_attr.find("file='") {
            let start_pos = start + 6;
            if let Some(end) = image_attr[start_pos..].find("'") {
                path = image_attr[start_pos..start_pos + end].to_string();
            }
        } else if let Some(start) = image_attr.find("file=\"") {
            let start_pos = start + 6;
            if let Some(end) = image_attr[start_pos..].find("\"") {
                path = image_attr[start_pos..start_pos + end].to_string();
            }
        } else {
            // 如果没有 file='...'，整个值就是文件路径
            path = image_attr.trim().to_string();
        }

        // 解析 dest='x1,y1,x2,y2'
        if let Some(start) = image_attr.find("dest='") {
            let start_pos = start + 6;
            if let Some(end) = image_attr[start_pos..].find("'") {
                let dest_str = &image_attr[start_pos..start_pos + end];
                let parts: Vec<&str> = dest_str.split(',').map(|s| s.trim()).collect();
                if parts.len() == 4 {
                    if let (Ok(x1), Ok(y1), Ok(x2), Ok(y2)) = (
                        parts[0].parse::<f32>(),
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                        parts[3].parse::<f32>(),
                    ) {
                        dest = Some((x1, y1, x2, y2));
                    }
                }
            }
        }

        // 解析 corner='x1,y1,x2,y2'
        if let Some(start) = image_attr.find("corner='") {
            let start_pos = start + 8;
            if let Some(end) = image_attr[start_pos..].find("'") {
                let corner_str = &image_attr[start_pos..start_pos + end];
                let parts: Vec<&str> = corner_str.split(',').map(|s| s.trim()).collect();
                if parts.len() == 4 {
                    if let (Ok(x1), Ok(y1), Ok(x2), Ok(y2)) = (
                        parts[0].parse::<f32>(),
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                        parts[3].parse::<f32>(),
                    ) {
                        corner = Some((x1, y1, x2, y2));
                    }
                }
            }
        }

        // 解析 fade='value' (0-255，转换为 0.0-1.0)
        if let Some(start) = image_attr.find("fade='") {
            let start_pos = start + 6;
            if let Some(end) = image_attr[start_pos..].find("'") {
                let fade_str = &image_attr[start_pos..start_pos + end];
                if let Ok(fade_val) = fade_str.parse::<u8>() {
                    fade = Some(fade_val as f32 / 255.0);
                }
            }
        }

        ImagePath {
            path,
            dest,
            corner,
            fade,
        }
    }

    fn get_custom_any<'a>(attributes: &'a ElementAttributes, keys: &[&str]) -> Option<&'a String> {
        keys.iter().find_map(|key| attributes.get_custom(key))
    }

    fn parse_box_sides(value: Option<&String>) -> (f32, f32, f32, f32) {
        value
            .map(|s| {
                let parts: Vec<f32> = s.split(',').filter_map(|v| v.trim().parse().ok()).collect();
                (
                    parts.get(0).copied().unwrap_or(0.0),
                    parts.get(1).copied().unwrap_or(0.0),
                    parts.get(2).copied().unwrap_or(0.0),
                    parts.get(3).copied().unwrap_or(0.0),
                )
            })
            .unwrap_or((0.0, 0.0, 0.0, 0.0))
    }

    fn parse_spacing_shorthand(value: Option<&String>) -> Option<(f32, f32, f32, f32)> {
        let value = value?;
        let parts: Vec<f32> = value
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|part| !part.is_empty())
            .filter_map(|part| part.trim().parse::<f32>().ok())
            .collect();
        match parts.as_slice() {
            [all] => Some((*all, *all, *all, *all)),
            [vertical, horizontal] => Some((*vertical, *horizontal, *vertical, *horizontal)),
            [top, horizontal, bottom] => Some((*top, *horizontal, *bottom, *horizontal)),
            [top, right, bottom, left] => Some((*top, *right, *bottom, *left)),
            _ => None,
        }
    }

    fn button_text_padding(element: &LayoutElement, text_align_str: &str) -> (f32, f32, f32, f32) {
        let (left, top, right, bottom) =
            Self::parse_box_sides(element.attributes.get_custom("textpadding"));
        let default_horizontal_padding = match text_align_str {
            "left" | "right" => 0.0,
            _ => 8.0,
        };
        let default_vertical_padding = 4.0;

        (
            if left > 0.0 {
                left
            } else {
                default_horizontal_padding
            },
            if top > 0.0 {
                top
            } else {
                default_vertical_padding
            },
            if right > 0.0 {
                right
            } else {
                default_horizontal_padding
            },
            if bottom > 0.0 {
                bottom
            } else {
                default_vertical_padding
            },
        )
    }

    fn resolve_inline_button_icon(
        element: &LayoutElement,
        enabled: bool,
        response: &Response,
    ) -> Option<InlineButtonIcon> {
        let id = element.attributes.id.as_deref().unwrap_or_default();
        let action = element
            .attributes
            .get_custom("action")
            .map(|s| s.as_str())
            .unwrap_or_default();
        let fallback_glyph = if id == "btnShowMore_wrap" || action.ends_with(":show") {
            Some(InlineButtonGlyph::ChevronDown)
        } else if id == "btnHideMore_wrap" || action.ends_with(":hide") {
            Some(InlineButtonGlyph::ChevronUp)
        } else {
            None
        };

        let icon_attr = if !enabled {
            Self::get_custom_any(
                &element.attributes,
                &["inlinedisabledicon", "inline-disabled-icon"],
            )
            .or_else(|| Self::get_custom_any(&element.attributes, &["inlineicon", "inline-icon"]))
        } else if response.is_pointer_button_down_on() {
            Self::get_custom_any(
                &element.attributes,
                &["inlinepushedicon", "inline-pushed-icon"],
            )
            .or_else(|| {
                Self::get_custom_any(&element.attributes, &["inlinehoticon", "inline-hot-icon"])
            })
            .or_else(|| Self::get_custom_any(&element.attributes, &["inlineicon", "inline-icon"]))
        } else if response.hovered() {
            Self::get_custom_any(&element.attributes, &["inlinehoticon", "inline-hot-icon"])
                .or_else(|| {
                    Self::get_custom_any(&element.attributes, &["inlineicon", "inline-icon"])
                })
        } else {
            Self::get_custom_any(&element.attributes, &["inlineicon", "inline-icon"])
        };

        let size = Self::get_custom_any(
            &element.attributes,
            &[
                "inlineiconsize",
                "inline-icon-size",
                "iconsize",
                "icon-size",
            ],
        )
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(12.0);
        let gap = Self::get_custom_any(
            &element.attributes,
            &["inlineicongap", "inline-icon-gap", "icongap", "icon-gap"],
        )
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(4.0);

        Some(InlineButtonIcon {
            glyph: icon_attr
                .map(|attr| InlineButtonGlyph::Image(Self::parse_image_path(attr)))
                .or(fallback_glyph)?,
            size,
            gap,
        })
    }

    fn compute_button_content_rect(rect: Rect, padding: (f32, f32, f32, f32)) -> Rect {
        Rect::from_min_max(
            Pos2::new(rect.min.x + padding.0, rect.min.y + padding.1),
            Pos2::new(rect.max.x - padding.2, rect.max.y - padding.3),
        )
    }

    fn compute_button_text_and_icon_rects(
        rect: Rect,
        padding: (f32, f32, f32, f32),
        inline_icon: Option<&InlineButtonIcon>,
    ) -> (Rect, Option<Rect>) {
        let content_rect = Self::compute_button_content_rect(rect, padding);
        if let Some(icon) = inline_icon {
            let icon_rect = Rect::from_min_size(
                Pos2::new(
                    content_rect.max.x - icon.size,
                    content_rect.center().y - icon.size / 2.0,
                ),
                Vec2::splat(icon.size),
            );
            let text_max_x = (icon_rect.min.x - icon.gap).max(content_rect.min.x);
            (
                Rect::from_min_max(content_rect.min, Pos2::new(text_max_x, content_rect.max.y)),
                Some(icon_rect),
            )
        } else {
            (content_rect, None)
        }
    }

    fn paint_image_in_rect(&mut self, ui: &mut Ui, image: &ImagePath, target_rect: Rect) {
        if let Some(texture) =
            self.resource_cache
                .get_background(ui.ctx(), &self.dpi_config, &image.path)
        {
            let uv_rect = if let Some((x1, y1, x2, y2)) = image.dest {
                let tex_size = texture.size();
                Rect::from_min_max(
                    Pos2::new(x1 / tex_size[0] as f32, y1 / tex_size[1] as f32),
                    Pos2::new(x2 / tex_size[0] as f32, y2 / tex_size[1] as f32),
                )
            } else {
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0))
            };

            let tint = if let Some(fade) = image.fade {
                Color32::from_rgba_unmultiplied(255, 255, 255, (fade * 255.0) as u8)
            } else {
                Color32::WHITE
            };

            ui.painter().image(texture.id(), target_rect, uv_rect, tint);
        }
    }

    fn paint_chevron_in_rect(ui: &mut Ui, rect: Rect, color: Color32, up: bool) {
        let inset = rect.width().min(rect.height()).min(12.0) * 0.18;
        let draw_rect = rect.shrink(inset.max(1.0));
        let stroke = egui::Stroke::new(1.4, color);
        let left = Pos2::new(
            draw_rect.left(),
            if up {
                draw_rect.bottom()
            } else {
                draw_rect.top()
            },
        );
        let mid = Pos2::new(
            draw_rect.center().x,
            if up {
                draw_rect.top()
            } else {
                draw_rect.bottom()
            },
        );
        let right = Pos2::new(
            draw_rect.right(),
            if up {
                draw_rect.bottom()
            } else {
                draw_rect.top()
            },
        );
        ui.painter().line_segment([left, mid], stroke);
        ui.painter().line_segment([mid, right], stroke);
    }

    fn paint_checkmark_in_rect(ui: &mut Ui, rect: Rect, color: Color32) {
        let draw_rect = rect.shrink(rect.width().min(rect.height()) * 0.18);
        let stroke = egui::Stroke::new(1.8, color);
        let left = Pos2::new(draw_rect.left(), draw_rect.center().y);
        let mid = Pos2::new(
            draw_rect.center().x - draw_rect.width() * 0.08,
            draw_rect.bottom(),
        );
        let right = Pos2::new(draw_rect.right(), draw_rect.top());
        ui.painter().line_segment([left, mid], stroke);
        ui.painter().line_segment([mid, right], stroke);
    }

    fn layout_button_text_galley(
        &self,
        ui: &Ui,
        text: String,
        font_id: egui::FontId,
        text_color: Color32,
        max_width: f32,
        wrap: bool,
    ) -> std::sync::Arc<egui::Galley> {
        let mut job = egui::text::LayoutJob::single_section(
            text,
            egui::TextFormat {
                font_id,
                color: text_color,
                ..Default::default()
            },
        );
        job.halign = egui::Align::Min;
        if wrap {
            job.wrap = egui::text::TextWrapping {
                max_width,
                ..Default::default()
            };
        }
        ui.painter().layout_job(job)
    }

    /// 创建新的布局渲染器
    pub fn new(dpi_config: DpiConfig, i18n_strings: HashMap<String, String>) -> Self {
        Self {
            dpi_config,
            style_engine: StyleEngine::new(),
            resource_cache: crate::ui::dpi_handler::ResourceCache::new(),
            i18n_strings,
            interaction_state: InteractionState::default(),
            taffy_bridge: TaffyBridge::new(),
        }
    }

    /// 创建带样式引擎的布局渲染器
    pub fn with_style_engine(
        dpi_config: DpiConfig,
        style_engine: StyleEngine,
        i18n_strings: HashMap<String, String>,
    ) -> Self {
        Self {
            dpi_config,
            style_engine,
            resource_cache: crate::ui::dpi_handler::ResourceCache::new(),
            i18n_strings,
            interaction_state: InteractionState::default(),
            taffy_bridge: TaffyBridge::new(),
        }
    }

    /// 渲染布局树
    pub fn render(&mut self, ui: &mut Ui, layout_tree: &LayoutTree) -> RenderResult {
        let mut result = RenderResult::new();

        // 应用全局样式
        self.style_engine.apply_global_style(ui.style_mut());

        // 检测是否使用新格式 (根元素有 flex_style)
        let has_flex = layout_tree.root.flex_style.is_some();
        if has_flex {
            self.render_with_taffy(ui, layout_tree, &mut result);
        } else {
            // 旧格式: 使用原有渲染路径
            self.render_element(ui, &layout_tree.root, &mut result);
        }

        result
    }

    /// 在无窗口模式下仅执行一次布局求解，返回结构化布局快照。
    pub fn compute_layout_snapshot(&mut self, layout_tree: &LayoutTree) -> Option<ComputedLayout> {
        if layout_tree.root.flex_style.is_none() {
            return None;
        }

        self.taffy_bridge
            .set_i18n_strings(self.i18n_strings.clone());
        Some(self.taffy_bridge.compute_layout(
            layout_tree,
            self.dpi_config.window_width,
            self.dpi_config.window_height,
        ))
    }

    /// 使用 Taffy 布局引擎渲染 (新格式)
    /// Taffy 在 1x 逻辑像素坐标系中计算布局，输出直接作为 egui 逻辑坐标
    fn render_with_taffy(
        &mut self,
        ui: &mut Ui,
        layout_tree: &LayoutTree,
        result: &mut RenderResult,
    ) {
        // Pass i18n strings to taffy bridge for text measurement
        self.taffy_bridge
            .set_i18n_strings(self.i18n_strings.clone());

        // 计算布局 (逻辑像素，尺寸来自配置，直接匹配 egui 逻辑空间)
        let computed = self.taffy_bridge.compute_layout(
            layout_tree,
            self.dpi_config.window_width,
            self.dpi_config.window_height,
        );

        let window_rect = ui.max_rect();
        self.render_taffy_node(ui, &layout_tree.root, &computed, window_rect, result);
    }

    /// 递归渲染 Taffy 计算后的节点
    fn render_taffy_node(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        computed: &ComputedLayout,
        window_rect: Rect,
        result: &mut RenderResult,
    ) {
        // 检查可见性
        let visible = element
            .visual_style
            .as_ref()
            .map_or(element.attributes.visible.unwrap_or(true), |vs| vs.visible);
        if !visible {
            return;
        }

        // 获取计算后的 egui::Rect
        let egui_rect = self.lookup_egui_rect(element, computed, window_rect);

        // 渲染当前元素的视觉部分 (背景、边框等)
        self.render_element_visual(ui, element, egui_rect);

        // 渲染控件内容 (按钮文字/图片、标签文字等)
        self.render_widget_at_rect(ui, element, egui_rect, result);

        // 递归渲染子元素（Select 的子元素由 ComboBox 内部渲染，不递归）
        if element.element_type != ElementType::Select {
            for child in &element.children {
                self.render_taffy_node(ui, child, computed, window_rect, result);
            }
        }
    }

    /// 查找元素对应的 egui::Rect
    fn lookup_egui_rect(
        &self,
        element: &LayoutElement,
        computed: &ComputedLayout,
        window_rect: Rect,
    ) -> Rect {
        let offset = window_rect.min;

        // 优先通过 ID 查找
        let id = element.attributes.id.as_deref().or_else(|| {
            element
                .widget_props
                .as_ref()
                .and_then(|wp| wp.id.as_deref())
        });

        if let Some(id_str) = id {
            if let Some(cr) = computed.get_rect(id_str) {
                return cr.to_egui_rect(offset);
            }
        }

        // 回退: 遍历 computed.nodes 找匹配的 element_type (无 ID 的容器)
        for node in &computed.nodes {
            if node.element_type == element.element_type && node.id.is_none() {
                return node.rect.to_egui_rect(offset);
            }
        }

        // 最终回退: 使用整个窗口
        window_rect
    }

    /// 渲染元素的视觉部分 (背景色/图、边框)
    fn render_element_visual(&mut self, ui: &mut Ui, element: &LayoutElement, rect: Rect) {
        let painter = ui.painter();

        // 背景色
        if let Some(vs) = &element.visual_style {
            if let Some(bg) = &vs.background {
                if let Some(color) = Self::parse_color_static(bg) {
                    let rounding = vs.border_radius.unwrap_or(0.0) as u8;
                    painter.rect_filled(rect, CornerRadius::same(rounding), color);
                }
            }
        } else if let Some(bg) = &element.attributes.background {
            // 只有纯色值才在这里渲染，图片路径由下面的背景图处理
            if !bg.contains("assets/")
                && !bg.ends_with(".png")
                && !bg.ends_with(".jpg")
                && !bg.contains("@2x")
            {
                if let Some(color) = Self::parse_color_static(bg) {
                    let corner_radius = self.get_corner_radius(element);
                    painter.rect_filled(rect, corner_radius, color);
                }
            }
        }

        // 背景图
        let bg_image = element
            .visual_style
            .as_ref()
            .and_then(|vs| vs.background_image.as_deref())
            .or_else(|| {
                element.attributes.background.as_deref().filter(|bg| {
                    bg.contains("assets/")
                        || bg.ends_with(".png")
                        || bg.ends_with(".jpg")
                        || bg.contains("@2x")
                })
            });

        if let Some(img_path) = bg_image {
            if let Some(texture) =
                self.resource_cache
                    .get_background(ui.ctx(), &self.dpi_config, img_path)
            {
                painter.image(
                    texture.id(),
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        }

        // 边框
        if let Some(vs) = &element.visual_style {
            if let (Some(border_color), Some(border_width)) = (&vs.border_color, vs.border_width) {
                if let Some(color) = Self::parse_color_static(border_color) {
                    let rounding = vs.border_radius.unwrap_or(0.0) as u8;
                    let stroke = egui::Stroke::new(border_width, color);
                    painter.rect(
                        rect,
                        CornerRadius::same(rounding),
                        Color32::TRANSPARENT,
                        stroke,
                        egui::StrokeKind::Outside,
                    );
                }
            }
        } else {
            // 旧格式边框
            if let Some(stroke) = self.get_border_stroke(element) {
                let corner_radius = self.get_corner_radius(element);
                painter.rect_stroke(
                    rect,
                    corner_radius,
                    stroke,
                    egui::epaint::StrokeKind::Outside,
                );
            }
        }
    }

    /// 在 Taffy 计算的 rect 内渲染控件
    fn render_widget_at_rect(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        rect: Rect,
        result: &mut RenderResult,
    ) {
        match &element.element_type {
            // 容器类型不需要额外控件渲染
            ElementType::Page
            | ElementType::VBox
            | ElementType::HBox
            | ElementType::Overlay
            | ElementType::Spacer
            | ElementType::Flex => {}

            ElementType::Button => self.render_button_at_rect(ui, element, rect, result),
            ElementType::Label => self.render_label_at_rect(ui, element, rect),
            ElementType::Checkbox => self.render_checkbox_at_rect(ui, element, rect, result),
            ElementType::Image => self.render_image_at_rect(ui, element, rect, result),
            ElementType::TextInput => self.render_text_input_at_rect(ui, element, rect, result),
            ElementType::ProgressBar => self.render_progress_at_rect(ui, element, rect),
            ElementType::Divider => self.render_divider_at_rect(ui, element, rect),
            ElementType::Select => self.render_select_at_rect(ui, element, rect, result),
        }
    }

    // =========================================================================
    //  Rect-based 控件渲染方法 (Taffy 新路径)
    // =========================================================================

    /// 在指定 rect 内渲染按钮
    fn render_button_at_rect(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        rect: Rect,
        result: &mut RenderResult,
    ) {
        let id = element
            .attributes
            .id
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("");
        let has_content_children = !element.children.is_empty();
        let text = if has_content_children {
            String::new()
        } else {
            self.get_display_text(&element.attributes)
        };
        let enabled = element.attributes.enabled.unwrap_or(true);

        let sense = if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        };
        let response = ui.interact(rect, egui::Id::new(id), sense);

        if ui.is_rect_visible(rect) {
            let inline_icon = if has_content_children {
                None
            } else {
                Self::resolve_inline_button_icon(element, enabled, &response)
            };
            let suppress_bg_image = matches!(
                inline_icon.as_ref().map(|icon| &icon.glyph),
                Some(InlineButtonGlyph::ChevronDown | InlineButtonGlyph::ChevronUp)
            );

            // 选择背景图片
            let normalimage = element.attributes.get_custom("normalimage");
            let hotimage = element.attributes.get_custom("hotimage");
            let pushedimage = element.attributes.get_custom("pushedimage");
            let disabledimage = element.attributes.get_custom("disabledimage");

            let bg_image = if suppress_bg_image {
                None
            } else if !enabled {
                disabledimage.or(normalimage)
            } else if response.is_pointer_button_down_on() {
                pushedimage.or(hotimage).or(normalimage)
            } else if response.hovered() {
                hotimage.or(normalimage)
            } else {
                normalimage
            };

            // 渲染背景图片
            if let Some(img_path_str) = bg_image {
                let image_path = Self::parse_image_path(img_path_str);
                if let Some((x1, y1, x2, y2)) = image_path.dest {
                    let dest_rect = Rect::from_min_max(
                        Pos2::new(rect.min.x + x1, rect.min.y + y1),
                        Pos2::new(rect.min.x + x2, rect.min.y + y2),
                    );
                    self.paint_image_in_rect(
                        ui,
                        &ImagePath {
                            dest: None,
                            ..image_path.clone()
                        },
                        dest_rect,
                    );
                } else {
                    self.paint_image_in_rect(ui, &image_path, rect);
                }
            } else {
                // No image: draw CSS border button (outline style)
                let border_radius = element
                    .visual_style
                    .as_ref()
                    .and_then(|vs| vs.border_radius)
                    .or_else(|| {
                        element
                            .attributes
                            .get_custom("borderround")
                            .and_then(|s| s.split(',').next()?.trim().parse::<f32>().ok())
                    })
                    .unwrap_or(0.0);
                let border_color_str = element
                    .visual_style
                    .as_ref()
                    .and_then(|vs| vs.border_color.clone())
                    .or_else(|| element.attributes.get_custom("border-color").cloned());
                let border_width = element
                    .visual_style
                    .as_ref()
                    .and_then(|vs| vs.border_width)
                    .unwrap_or(1.0);
                let bg_color = element
                    .visual_style
                    .as_ref()
                    .and_then(|vs| vs.background.as_ref())
                    .or_else(|| element.attributes.background.as_ref());

                // Background fill (if specified)
                if let Some(bg) = bg_color {
                    if let Some(color) = Self::parse_color_static(bg) {
                        let rounding = CornerRadius::same(border_radius as u8);
                        ui.painter().rect_filled(rect, rounding, color);
                    }
                }

                // Hover effect: lighten border/background
                if response.hovered() && enabled {
                    let hover_bg = element.attributes.get_custom("hover-background");
                    if let Some(hbg) = hover_bg {
                        if let Some(color) = Self::parse_color_static(hbg) {
                            let rounding = CornerRadius::same(border_radius as u8);
                            ui.painter().rect_filled(rect, rounding, color);
                        }
                    }
                }

                // Border stroke
                if let Some(bc_str) = &border_color_str {
                    if let Some(bc) = Self::parse_color_static(bc_str) {
                        let rounding = CornerRadius::same(border_radius as u8);
                        let stroke_color = if response.hovered() && enabled {
                            // Brighter on hover
                            element
                                .attributes
                                .get_custom("hover-border-color")
                                .and_then(|c| Self::parse_color_static(c))
                                .unwrap_or(bc)
                        } else {
                            bc
                        };
                        ui.painter().rect_stroke(
                            rect,
                            rounding,
                            egui::Stroke::new(border_width, stroke_color),
                            egui::epaint::StrokeKind::Inside,
                        );
                    }
                }
            }

            // 文本颜色 (按状态)
            let text_color = if !enabled {
                element
                    .attributes
                    .get_custom("disabledtextcolor")
                    .and_then(|c| self.parse_color(c))
                    .unwrap_or(Color32::GRAY)
            } else if response.is_pointer_button_down_on() {
                element
                    .attributes
                    .get_custom("pushedtextcolor")
                    .and_then(|c| self.parse_color(c))
                    .or_else(|| {
                        element
                            .attributes
                            .get_custom("hottextcolor")
                            .and_then(|c| self.parse_color(c))
                    })
                    .or_else(|| {
                        element
                            .attributes
                            .color
                            .as_ref()
                            .and_then(|c| self.parse_color(c))
                    })
                    .unwrap_or(Color32::WHITE)
            } else if response.hovered() {
                element
                    .attributes
                    .get_custom("hover-color")
                    .or_else(|| element.attributes.get_custom("hottextcolor"))
                    .and_then(|c| self.parse_color(c))
                    .or_else(|| {
                        element
                            .attributes
                            .color
                            .as_ref()
                            .and_then(|c| self.parse_color(c))
                    })
                    .unwrap_or(Color32::WHITE)
            } else {
                element
                    .attributes
                    .color
                    .as_ref()
                    .and_then(|c| self.parse_color(c))
                    .unwrap_or(Color32::WHITE)
            };

            if !has_content_children {
                let font_id = self.get_font_id(element);
                let text_align_str = element
                    .attributes
                    .get_custom("textalign")
                    .map(|s| s.as_str())
                    .or_else(|| element.attributes.align.as_deref())
                    .unwrap_or("center");
                let padding = Self::button_text_padding(element, text_align_str);
                let (text_rect, icon_rect) =
                    Self::compute_button_text_and_icon_rects(rect, padding, inline_icon.as_ref());

                let has_width_constraint = element
                    .flex_style
                    .as_ref()
                    .map(|fs| {
                        !matches!(fs.max_width, crate::layout::dimension::Dimension::Auto)
                            || !matches!(fs.width, crate::layout::dimension::Dimension::Auto)
                    })
                    .unwrap_or_else(|| {
                        element.attributes.width.is_some() || element.attributes.max_width.is_some()
                    });

                if has_width_constraint {
                    let galley = self.layout_button_text_galley(
                        ui,
                        text.clone(),
                        font_id.clone(),
                        text_color,
                        text_rect.width(),
                        true,
                    );
                    let galley_size = galley.size();
                    let x = match text_align_str {
                        "left" => text_rect.min.x,
                        "right" => text_rect.max.x - galley_size.x,
                        _ => text_rect.min.x + (text_rect.width() - galley_size.x) / 2.0,
                    };
                    let y = text_rect.min.y + (text_rect.height() - galley_size.y) / 2.0;
                    ui.painter().galley(Pos2::new(x, y), galley, text_color);
                } else {
                    let galley = self.layout_button_text_galley(
                        ui,
                        text.clone(),
                        font_id.clone(),
                        text_color,
                        text_rect.width(),
                        false,
                    );
                    let galley_size = galley.size();
                    let text_pos = match text_align_str {
                        "left" => {
                            Pos2::new(text_rect.min.x, text_rect.center().y - galley_size.y / 2.0)
                        }
                        "right" => Pos2::new(
                            text_rect.max.x - galley_size.x,
                            text_rect.center().y - galley_size.y / 2.0,
                        ),
                        _ => Pos2::new(
                            text_rect.center().x - galley_size.x / 2.0,
                            text_rect.center().y - galley_size.y / 2.0,
                        ),
                    };
                    ui.painter().galley(text_pos, galley, text_color);
                }

                if let (Some(icon), Some(icon_rect)) = (inline_icon.as_ref(), icon_rect) {
                    match &icon.glyph {
                        InlineButtonGlyph::Image(image) => {
                            self.paint_image_in_rect(ui, image, icon_rect)
                        }
                        InlineButtonGlyph::ChevronDown => {
                            Self::paint_chevron_in_rect(ui, icon_rect, text_color, false)
                        }
                        InlineButtonGlyph::ChevronUp => {
                            Self::paint_chevron_in_rect(ui, icon_rect, text_color, true)
                        }
                    }
                }
            }
        }

        // hover 光标
        if response.hovered() && enabled {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        if response.clicked() && enabled {
            result.button_clicks.insert(id.to_string(), true);
            // Capture action attribute if present
            if let Some(action) = element.attributes.get_custom("action") {
                result.button_actions.insert(id.to_string(), action.clone());
            }
        }
        result.button_responses.insert(id.to_string(), response);
    }

    /// 在指定 rect 内渲染标签
    fn render_label_at_rect(&mut self, ui: &mut Ui, element: &LayoutElement, rect: Rect) {
        if !ui.is_rect_visible(rect) {
            return;
        }

        let text = self.get_display_text(&element.attributes);
        let font_id = self.get_font_id(element);
        let text_color = element
            .attributes
            .color
            .as_ref()
            .and_then(|c| self.parse_color(c))
            .unwrap_or(Color32::WHITE);

        let text_align_str = element
            .attributes
            .get_custom("textalign")
            .map(|s| s.as_str())
            .or_else(|| element.attributes.align.as_deref())
            .unwrap_or("left");
        let valign = element
            .attributes
            .get_custom("valign")
            .map(|s| s.as_str())
            .unwrap_or("top");

        // 当元素有宽度约束（max-width 或显式 width）且文本超过 rect 宽度时启用换行
        let wrap_enabled = element.attributes.wrap.unwrap_or(false)
            || element
                .widget_props
                .as_ref()
                .and_then(|wp| wp.wrap)
                .unwrap_or(false);
        let has_width_constraint = wrap_enabled
            || element
                .flex_style
                .as_ref()
                .map(|fs| !matches!(fs.max_width, crate::layout::dimension::Dimension::Auto))
                .unwrap_or(false);

        if has_width_constraint {
            // 先测量单行宽度。短文案应继续走非换行路径，这样 RIGHT/CENTER
            // 对齐会真正贴到文本框右侧/中间，而不是被 wrap 路径拉回左侧。
            let single_line_galley =
                ui.painter()
                    .layout_no_wrap(text.clone(), font_id.clone(), text_color);

            if single_line_galley.size().x <= rect.width() {
                let (text_pos, align2) = match (text_align_str, valign) {
                    ("left", "center") | ("left", "vcenter") => (
                        Pos2::new(rect.min.x, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                    ),
                    ("left", "bottom") => {
                        (Pos2::new(rect.min.x, rect.max.y), egui::Align2::LEFT_BOTTOM)
                    }
                    ("center", "top") => (
                        Pos2::new(rect.center().x, rect.min.y),
                        egui::Align2::CENTER_TOP,
                    ),
                    ("center", "center") | ("center", "vcenter") => {
                        (rect.center(), egui::Align2::CENTER_CENTER)
                    }
                    ("center", "bottom") => (
                        Pos2::new(rect.center().x, rect.max.y),
                        egui::Align2::CENTER_BOTTOM,
                    ),
                    ("right", "top") => {
                        (Pos2::new(rect.max.x, rect.min.y), egui::Align2::RIGHT_TOP)
                    }
                    ("right", "center") | ("right", "vcenter") => (
                        Pos2::new(rect.max.x, rect.center().y),
                        egui::Align2::RIGHT_CENTER,
                    ),
                    ("right", "bottom") => (rect.max, egui::Align2::RIGHT_BOTTOM),
                    _ => (rect.min, egui::Align2::LEFT_TOP),
                };

                ui.painter()
                    .text(text_pos, align2, &text, font_id, text_color);
            } else {
                let mut job = egui::text::LayoutJob::single_section(
                    text.clone(),
                    egui::TextFormat {
                        font_id: font_id.clone(),
                        color: text_color,
                        ..Default::default()
                    },
                );
                job.halign = match text_align_str {
                    "center" => egui::Align::Center,
                    "right" => egui::Align::Max,
                    _ => egui::Align::Min,
                };
                job.wrap = egui::text::TextWrapping {
                    max_width: rect.width(),
                    ..Default::default()
                };

                let galley = ui.painter().layout_job(job);
                let galley_size = galley.size();

                let y = match valign {
                    "center" | "vcenter" => rect.min.y + (rect.height() - galley_size.y) / 2.0,
                    "bottom" => rect.max.y - galley_size.y,
                    _ => rect.min.y,
                };
                ui.painter()
                    .galley(Pos2::new(rect.min.x, y), galley, text_color);
            }
        } else {
            // 无约束：原始不换行渲染
            let (text_pos, align2) = match (text_align_str, valign) {
                ("left", "center") | ("left", "vcenter") => (
                    Pos2::new(rect.min.x, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                ),
                ("left", "bottom") => {
                    (Pos2::new(rect.min.x, rect.max.y), egui::Align2::LEFT_BOTTOM)
                }
                ("center", "top") => (
                    Pos2::new(rect.center().x, rect.min.y),
                    egui::Align2::CENTER_TOP,
                ),
                ("center", "center") | ("center", "vcenter") => {
                    (rect.center(), egui::Align2::CENTER_CENTER)
                }
                ("center", "bottom") => (
                    Pos2::new(rect.center().x, rect.max.y),
                    egui::Align2::CENTER_BOTTOM,
                ),
                ("right", "top") => (Pos2::new(rect.max.x, rect.min.y), egui::Align2::RIGHT_TOP),
                ("right", "center") | ("right", "vcenter") => (
                    Pos2::new(rect.max.x, rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                ),
                ("right", "bottom") => (rect.max, egui::Align2::RIGHT_BOTTOM),
                _ => (rect.min, egui::Align2::LEFT_TOP),
            };
            ui.painter()
                .text(text_pos, align2, &text, font_id, text_color);
        }
    }

    /// 在指定 rect 内渲染复选框
    fn render_checkbox_at_rect(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        rect: Rect,
        result: &mut RenderResult,
    ) {
        let id = element
            .attributes
            .id
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("");
        let text = self.get_display_text(&element.attributes);
        let enabled = element.attributes.enabled.unwrap_or(true);

        let response = ui.interact(rect, egui::Id::new(id), egui::Sense::click());

        // 初始化 checkbox 状态: 优先用交互状态, 否则从 XML checked 属性读取
        let xml_default = element
            .widget_props
            .as_ref()
            .and_then(|wp| wp.checked)
            .unwrap_or(false);
        let current_checked = *self
            .interaction_state
            .checkbox_states
            .entry(id.to_string())
            .or_insert(xml_default);

        if ui.is_rect_visible(rect) {
            let normalimage = element.attributes.get_custom("normalimage");
            let normalhotimage = element.attributes.get_custom("normalhotimage");
            let selectedimage = element.attributes.get_custom("selectedimage");
            let selectedhotimage = element.attributes.get_custom("selectedhotimage");
            let disabledimage = element.attributes.get_custom("disabledimage");

            let is_hovered = response.hovered();
            let checkbox_image = if !enabled {
                disabledimage.or(normalimage)
            } else if current_checked {
                if is_hovered {
                    selectedhotimage
                        .or(selectedimage)
                        .or(normalhotimage)
                        .or(normalimage)
                } else {
                    selectedimage.or(normalimage)
                }
            } else {
                if is_hovered {
                    normalhotimage.or(normalimage)
                } else {
                    normalimage
                }
            };

            // 复选框图标尺寸: 从纹理推算逻辑尺寸 (get_render_size 会将 @2x 纹理除 2)
            let checkbox_size = normalimage
                .and_then(|img_str| {
                    let ip = Self::parse_image_path(img_str);
                    self.resource_cache
                        .get_background(ui.ctx(), &self.dpi_config, &ip.path)
                        .map(|tex| self.dpi_config.get_render_size(tex).y)
                })
                .unwrap_or(rect.height().min(16.0));
            // 图标垂直居中
            let icon_y = rect.min.y + (rect.height() - checkbox_size) / 2.0;
            let checkbox_rect =
                Rect::from_min_size(Pos2::new(rect.min.x, icon_y), Vec2::splat(checkbox_size));

            if let Some(img_path_str) = checkbox_image {
                let image_path = Self::parse_image_path(img_path_str);
                if let Some(texture) =
                    self.resource_cache
                        .get_background(ui.ctx(), &self.dpi_config, &image_path.path)
                {
                    let full_uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                    if let Some((x1, y1, x2, y2)) = image_path.dest {
                        // dest = checkbox rect 内的目标绘制子区域
                        let dest_rect = Rect::from_min_max(
                            Pos2::new(checkbox_rect.min.x + x1, checkbox_rect.min.y + y1),
                            Pos2::new(checkbox_rect.min.x + x2, checkbox_rect.min.y + y2),
                        );
                        ui.painter()
                            .image(texture.id(), dest_rect, full_uv, Color32::WHITE);
                    } else {
                        ui.painter()
                            .image(texture.id(), checkbox_rect, full_uv, Color32::WHITE);
                    }
                } else {
                    // 回退: 简单方框 (图片加载失败)
                    ui.painter().rect_stroke(
                        checkbox_rect,
                        CornerRadius::same(2),
                        egui::Stroke::new(1.0, Color32::GRAY),
                        egui::epaint::StrokeKind::Outside,
                    );
                    if current_checked {
                        ui.painter().rect_filled(
                            checkbox_rect.shrink(3.0),
                            CornerRadius::same(1),
                            Color32::WHITE,
                        );
                    }
                }
            } else {
                // 回退: 无图片配置
                ui.painter().rect_stroke(
                    checkbox_rect,
                    CornerRadius::same(2),
                    egui::Stroke::new(1.0, Color32::GRAY),
                    egui::epaint::StrokeKind::Outside,
                );
                if current_checked {
                    ui.painter().rect_filled(
                        checkbox_rect.shrink(3.0),
                        CornerRadius::same(1),
                        Color32::WHITE,
                    );
                }
            }

            // 文字 (在复选框右侧)
            let text_padding_left = element
                .attributes
                .get_custom("textpadding")
                .and_then(|s| {
                    s.split(',')
                        .next()
                        .and_then(|v| v.trim().parse::<f32>().ok())
                })
                .unwrap_or(checkbox_size + 4.0);
            let font_size = self.get_font_id(element).size;
            let text_color = element
                .attributes
                .color
                .as_ref()
                .and_then(|c| self.parse_color(c))
                .unwrap_or(Color32::WHITE);

            // 解析带链接的文本
            let segments = self.parse_text_with_links(&text);
            let link_color = element
                .attributes
                .get_custom("linkcolor")
                .and_then(|c| self.parse_color(c))
                .unwrap_or(Color32::from_rgb(0, 196, 178)); // TapTap green default

            // 文本区域：checkbox 图标右侧到 rect 右边
            let text_left = rect.min.x + text_padding_left;
            let text_max_width = (rect.max.x - text_left).max(0.0);

            if segments
                .iter()
                .any(|s| matches!(s, TextSegment::Link { .. }))
            {
                // 有链接时：用 LayoutJob 支持换行 + 多色段
                let font_id = egui::FontId::proportional(font_size);

                // 记录每个 link 在 job.text 中的 char offset 范围
                let mut link_char_ranges: Vec<(std::ops::Range<usize>, String)> = Vec::new();
                let mut job = egui::text::LayoutJob::default();
                job.wrap = egui::text::TextWrapping {
                    max_width: text_max_width,
                    ..Default::default()
                };

                for segment in &segments {
                    match segment {
                        TextSegment::Text(t) => {
                            job.append(
                                t,
                                0.0,
                                egui::TextFormat {
                                    font_id: font_id.clone(),
                                    color: text_color,
                                    ..Default::default()
                                },
                            );
                        }
                        TextSegment::Link {
                            id: link_id,
                            text: link_text,
                        } => {
                            let char_start = job.text.chars().count();
                            job.append(
                                link_text,
                                0.0,
                                egui::TextFormat {
                                    font_id: font_id.clone(),
                                    color: link_color,
                                    underline: egui::Stroke::NONE,
                                    ..Default::default()
                                },
                            );
                            let char_end = job.text.chars().count();
                            link_char_ranges.push((char_start..char_end, link_id.clone()));
                        }
                    }
                }

                let galley = ui.painter().layout_job(job);
                let galley_height = galley.size().y;
                // 垂直居中
                let text_y = rect.min.y + (rect.height() - galley_height) / 2.0;
                let text_origin = Pos2::new(text_left, text_y);

                // 绘制文本
                ui.painter().galley(text_origin, galley.clone(), text_color);

                // 链接点击检测：用 CCursor + pos_from_cursor 获取链接区域
                for (char_range, link_id) in &link_char_ranges {
                    let start_rect = galley.pos_from_cursor(egui::epaint::text::cursor::CCursor {
                        index: char_range.start,
                        prefer_next_row: false,
                    });
                    let end_rect = galley.pos_from_cursor(egui::epaint::text::cursor::CCursor {
                        index: char_range.end,
                        prefer_next_row: true,
                    });

                    // 链接区域 (可能跨多行，用整体边界框)
                    let link_rect = Rect::from_min_max(
                        Pos2::new(
                            text_origin.x + start_rect.min.x,
                            text_origin.y + start_rect.min.y,
                        ),
                        Pos2::new(
                            text_origin.x + end_rect.max.x,
                            text_origin.y + end_rect.max.y,
                        ),
                    );

                    let link_resp = ui.interact(
                        link_rect,
                        egui::Id::new(format!("link_{}", link_id)),
                        egui::Sense::click(),
                    );
                    if link_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        // 下划线 (最后一行底部)
                        ui.painter().line_segment(
                            [Pos2::new(link_rect.min.x, link_rect.max.y), link_rect.max],
                            egui::Stroke::new(1.0, link_color),
                        );
                    }
                    if link_resp.clicked() {
                        result.link_clicks.insert(link_id.clone(), true);
                    }
                }
            } else {
                // 纯文本也使用 LayoutJob 支持换行
                let font_id = egui::FontId::proportional(font_size);
                let mut job = egui::text::LayoutJob::single_section(
                    text.clone(),
                    egui::TextFormat {
                        font_id,
                        color: text_color,
                        ..Default::default()
                    },
                );
                job.wrap = egui::text::TextWrapping {
                    max_width: text_max_width,
                    ..Default::default()
                };
                let galley = ui.painter().layout_job(job);
                let galley_height = galley.size().y;
                let text_y = rect.min.y + (rect.height() - galley_height) / 2.0;
                ui.painter()
                    .galley(Pos2::new(text_left, text_y), galley, text_color);
            }
        }

        if response.clicked() {
            let new_checked = !current_checked;
            self.interaction_state
                .checkbox_states
                .insert(id.to_string(), new_checked);
            result.checkbox_changes.insert(id.to_string(), new_checked);
        }
        result.checkbox_responses.insert(id.to_string(), response);
    }

    /// 在指定 rect 内渲染图片
    fn render_image_at_rect(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        rect: Rect,
        result: &mut RenderResult,
    ) {
        if !ui.is_rect_visible(rect) {
            return;
        }
        if let Some(icon) = &element.attributes.icon {
            let trim_alpha = element
                .attributes
                .get_custom("trim")
                .map(|value| value.eq_ignore_ascii_case("alpha"))
                .unwrap_or(false);
            let offset_x = element
                .attributes
                .get_custom("offset-x")
                .and_then(|value| value.parse::<f32>().ok())
                .unwrap_or(0.0);
            let offset_y = element
                .attributes
                .get_custom("offset-y")
                .and_then(|value| value.parse::<f32>().ok())
                .unwrap_or(0.0);
            let visible_uv_rect = if trim_alpha {
                self.resource_cache
                    .get_background_visible_uv(&self.dpi_config, icon)
                    .unwrap_or(Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)))
            } else {
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0))
            };
            if let Some(texture) =
                self.resource_cache
                    .get_background(ui.ctx(), &self.dpi_config, icon)
            {
                let texture_id = texture.id();
                let draw_rect = if trim_alpha {
                    let visible_center = visible_uv_rect.center();
                    let full_center = Pos2::new(0.5, 0.5);
                    let delta = visible_center - full_center;
                    rect.translate(Vec2::new(-delta.x * rect.width(), -delta.y * rect.height()))
                } else {
                    rect
                }
                .translate(Vec2::new(offset_x, offset_y));
                ui.painter().image(
                    texture_id,
                    draw_rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        }
        // Image 支持 action 属性点击（如箭头图标）
        if let Some(action) = element.attributes.get_custom("action") {
            let id = element
                .attributes
                .id
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("img");
            let response = ui.interact(rect, egui::Id::new(id), egui::Sense::click());
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.clicked() {
                result.button_clicks.insert(id.to_string(), true);
                result.button_actions.insert(id.to_string(), action.clone());
            }
        }
    }

    /// 在指定 rect 内渲染文本输入框
    fn render_text_input_at_rect(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        rect: Rect,
        result: &mut RenderResult,
    ) {
        let id = element
            .attributes
            .id
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("");
        let enabled = element.attributes.enabled.unwrap_or(true);
        let readonly = element
            .attributes
            .get_custom("readonly")
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false);
        let multiline = element
            .attributes
            .get_custom("multiline")
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false);
        let font_id = self.get_font_id(element);

        // 背景
        if let Some(bg_color) = element
            .attributes
            .background
            .as_ref()
            .and_then(|c| self.parse_color(c))
        {
            let corner_radius = self.get_corner_radius(element);
            ui.painter().rect_filled(rect, corner_radius, bg_color);
            if let Some(stroke) = self.get_border_stroke(element) {
                ui.painter().rect_stroke(
                    rect,
                    corner_radius,
                    stroke,
                    egui::epaint::StrokeKind::Outside,
                );
            }
        }

        // 先读颜色 (避免借用冲突)
        let text_color = element
            .attributes
            .color
            .as_ref()
            .and_then(|c| self.parse_color(c))
            .unwrap_or(Color32::WHITE);

        let icon_width = element
            .attributes
            .get_custom("icon-width")
            .or_else(|| element.attributes.get_custom("icon_width"))
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(16.0);
        let icon_height = element
            .attributes
            .get_custom("icon-height")
            .or_else(|| element.attributes.get_custom("icon_height"))
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(icon_width);
        let icon_gap = element
            .attributes
            .get_custom("icon-gap")
            .or_else(|| element.attributes.get_custom("icon_gap"))
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(10.0);
        let has_icon = element.attributes.icon.is_some();
        let (padding_top, padding_right, padding_bottom, padding_left) =
            element.attributes.padding.unwrap_or((0.0, 0.0, 0.0, 0.0));
        let right_padding = if has_icon {
            padding_right + icon_width + icon_gap
        } else {
            padding_right
        };

        let text_value = self
            .interaction_state
            .text_inputs
            .entry(id.to_string())
            .or_insert_with(String::new);
        let inner_rect = Rect::from_min_max(
            Pos2::new(rect.min.x + padding_left, rect.min.y + padding_top),
            Pos2::new(rect.max.x - right_padding, rect.max.y - padding_bottom),
        );
        if readonly {
            let display_value = text_value.clone();
            let response = ui.interact(
                rect,
                egui::Id::new(format!("readonly_text_input_{}", id)),
                egui::Sense::click(),
            );
            let galley = self.layout_button_text_galley(
                ui,
                display_value,
                font_id.clone(),
                text_color,
                inner_rect.width().max(1.0),
                false,
            );
            let text_pos = Pos2::new(inner_rect.min.x, rect.center().y - (galley.size().y / 2.0));
            ui.painter().galley(text_pos, galley, text_color);
            result.text_input_responses.insert(id.to_string(), response);
        } else {
            let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(inner_rect));
            child_ui.style_mut().visuals.extreme_bg_color = Color32::TRANSPARENT;

            let mut text_input = if multiline {
                egui::TextEdit::multiline(text_value)
            } else {
                egui::TextEdit::singleline(text_value)
            };
            text_input = text_input.font(font_id).text_color(text_color).frame(false);
            if !enabled {
                text_input = text_input.interactive(false);
            }
            text_input = text_input.desired_width(inner_rect.width());

            let response = child_ui.add(text_input);
            if response.changed() {
                result.text_input_changes.insert(
                    id.to_string(),
                    self.interaction_state
                        .text_inputs
                        .get(id)
                        .cloned()
                        .unwrap_or_default(),
                );
            }
            result.text_input_responses.insert(id.to_string(), response);
        }

        if let Some(icon) = &element.attributes.icon {
            let icon_rect = Rect::from_center_size(
                Pos2::new(
                    rect.max.x - padding_right - icon_width / 2.0,
                    rect.center().y,
                ),
                Vec2::new(icon_width, icon_height),
            );
            if let Some(texture) =
                self.resource_cache
                    .get_background(ui.ctx(), &self.dpi_config, icon)
            {
                let render_size = self.dpi_config.get_render_size(texture);
                let draw_rect = Rect::from_center_size(
                    icon_rect.center(),
                    Vec2::new(
                        render_size.x.min(icon_rect.width()),
                        render_size.y.min(icon_rect.height()),
                    ),
                );
                ui.painter().image(
                    texture.id(),
                    draw_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        }
    }

    /// 在指定 rect 内渲染进度条
    fn render_progress_at_rect(&mut self, ui: &mut Ui, element: &LayoutElement, rect: Rect) {
        if !ui.is_rect_visible(rect) {
            return;
        }

        let min = element
            .attributes
            .get_custom("min")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);
        let max = element
            .attributes
            .get_custom("max")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(100.0);
        let value = element
            .attributes
            .get_custom("value")
            .and_then(|s| s.parse::<f32>().ok())
            .or_else(|| element.attributes.progress.map(|p| p * (max - min) + min))
            .unwrap_or(0.0);
        let progress = if max > min {
            ((value - min) / (max - min)).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let corner_radius = self.get_corner_radius(element);
        // forecolor 用于前景纯色回退
        let fore_color = element
            .attributes
            .get_custom("forecolor")
            .and_then(|c| self.parse_color(c))
            .or_else(|| {
                element
                    .attributes
                    .color
                    .as_ref()
                    .and_then(|c| self.parse_color(c))
            });

        // 背景
        if let Some(bg_color) = element
            .attributes
            .background
            .as_ref()
            .and_then(|c| self.parse_color(c))
        {
            ui.painter().rect_filled(rect, corner_radius, bg_color);
        }

        // 前景
        let progress_width = rect.width() * progress;
        if progress_width > 0.0 {
            let progress_rect =
                Rect::from_min_size(rect.min, Vec2::new(progress_width, rect.height()));

            let foreimage_str = element
                .attributes
                .get_custom("foreimage")
                .cloned()
                .or_else(|| element.attributes.get_custom("bar-image").cloned());
            if let Some(foreimage_str) = foreimage_str {
                let image_path = Self::parse_image_path(&foreimage_str);
                if let Some(fg_texture) =
                    self.resource_cache
                        .get_background(ui.ctx(), &self.dpi_config, &image_path.path)
                {
                    // 进度条前景: 按进度裁剪 UV 的 x 轴
                    let uv_rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(progress, 1.0));
                    ui.painter()
                        .image(fg_texture.id(), progress_rect, uv_rect, Color32::WHITE);
                } else if let Some(fc) = fore_color {
                    ui.painter().rect_filled(progress_rect, corner_radius, fc);
                }
            } else if let Some(fc) = fore_color {
                ui.painter().rect_filled(progress_rect, corner_radius, fc);
            }
        }
    }

    /// 在指定 rect 内渲染分隔线
    fn render_divider_at_rect(&mut self, ui: &mut Ui, element: &LayoutElement, rect: Rect) {
        if let Some(color) = element
            .attributes
            .color
            .as_ref()
            .and_then(|c| self.parse_color(c))
            .or_else(|| {
                element
                    .attributes
                    .background
                    .as_ref()
                    .and_then(|c| self.parse_color(c))
            })
        {
            let corner_radius = self.get_corner_radius(element);
            ui.painter().rect_filled(rect, corner_radius, color);
        }
    }

    /// Render a Select/Dropdown control at the given rect
    /// XML: <Select id="langSelect" action="switch_language" selected="zh-CN">
    ///        <Option value="zh-CN" text="简体中文" />
    ///        <Option value="en-US" text="English" />
    ///      </Select>
    fn render_select_at_rect(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        rect: Rect,
        result: &mut RenderResult,
    ) {
        let id = element
            .attributes
            .id
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("select");
        let action = element
            .attributes
            .get_custom("action")
            .cloned()
            .unwrap_or_default();

        // Get current selected value from interaction state or attribute
        let selected = self
            .interaction_state
            .text_inputs
            .get(id)
            .cloned()
            .or_else(|| element.attributes.get_custom("selected").cloned())
            .unwrap_or_default();

        // Build options from children: <Option value="xx" text="Display" />
        let mut options: Vec<(String, String)> = Vec::new();
        for child in &element.children {
            let value = child
                .attributes
                .get_custom("value")
                .cloned()
                .unwrap_or_default();
            let text = self.get_display_text(&child.attributes);
            if !value.is_empty() {
                options.push((value, text));
            }
        }

        // Find display text for current selection
        let display_text = options
            .iter()
            .find(|(v, _)| v == &selected)
            .map(|(_, t)| t.clone())
            .unwrap_or(selected.clone());

        // Style
        let font_size = element
            .attributes
            .get_custom("font_size")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(13.0);
        let text_color = element
            .attributes
            .color
            .as_ref()
            .and_then(|c| Self::parse_color_static(c))
            .unwrap_or(Color32::WHITE);
        let select_id = egui::Id::new(format!("select_{}", id));
        let response = ui.interact(rect, select_id, egui::Sense::click());
        let is_open = self
            .interaction_state
            .select_open_states
            .get(id)
            .copied()
            .unwrap_or(false);

        if ui.is_rect_visible(rect) {
            if response.hovered() {
                ui.painter()
                    .rect_filled(rect, CornerRadius::same(8), Color32::from_white_alpha(8));
            }

            let content_padding_left = 10.0;
            let content_padding_right = 8.0;
            let icon_width = element
                .attributes
                .get_custom("dropdown-icon-width")
                .or_else(|| element.attributes.get_custom("dropdown_icon_width"))
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(10.0);
            let icon_height = element
                .attributes
                .get_custom("dropdown-icon-height")
                .or_else(|| element.attributes.get_custom("dropdown_icon_height"))
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(icon_width);
            let _image_width = element
                .attributes
                .get_custom("dropdown-image-width")
                .or_else(|| element.attributes.get_custom("dropdown_image_width"))
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(icon_width);
            let _image_height = element
                .attributes
                .get_custom("dropdown-image-height")
                .or_else(|| element.attributes.get_custom("dropdown_image_height"))
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(icon_height);
            let icon_gap = 4.0;
            let icon_rect = Rect::from_center_size(
                Pos2::new(
                    rect.max.x - content_padding_right - (icon_width / 2.0),
                    rect.center().y,
                ),
                Vec2::new(icon_width, icon_height),
            );
            let text_rect = Rect::from_min_max(
                Pos2::new(rect.min.x + content_padding_left, rect.min.y),
                Pos2::new(icon_rect.min.x - icon_gap, rect.max.y),
            );
            let galley = self.layout_button_text_galley(
                ui,
                display_text.clone(),
                egui::FontId::proportional(font_size),
                text_color,
                text_rect.width().max(1.0),
                false,
            );
            let galley_size = galley.size();
            let text_pos = Pos2::new(
                text_rect.min.x,
                text_rect.center().y - (galley_size.y / 2.0),
            );
            ui.painter().galley(text_pos, galley, text_color);

            let dropdown_image = if is_open {
                element
                    .attributes
                    .get_custom("dropdown-open-image")
                    .or_else(|| element.attributes.get_custom("dropdown_open_image"))
                    .or_else(|| {
                        element
                            .attributes
                            .get_custom("dropdown-image")
                            .or_else(|| element.attributes.get_custom("dropdown_image"))
                    })
            } else {
                element
                    .attributes
                    .get_custom("dropdown-image")
                    .or_else(|| element.attributes.get_custom("dropdown_image"))
            };

            if let Some(dropdown_image) = dropdown_image {
                let image_path = Self::parse_image_path(dropdown_image);
                if let Some(texture) =
                    self.resource_cache
                        .get_background(ui.ctx(), &self.dpi_config, &image_path.path)
                {
                    let natural_size = self.dpi_config.get_render_size(texture);
                    let draw_size = Vec2::new(
                        natural_size.x.min(icon_rect.width()),
                        natural_size.y.min(icon_rect.height()),
                    );
                    let draw_rect = Rect::from_center_size(icon_rect.center(), draw_size);
                    ui.painter().image(
                        texture.id(),
                        draw_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    Self::paint_chevron_in_rect(ui, icon_rect, text_color, is_open);
                }
            } else {
                Self::paint_chevron_in_rect(ui, icon_rect, text_color, is_open);
            }
        }

        let mut new_selected = selected.clone();
        let mut new_open_state = is_open;
        if response.clicked() {
            new_open_state = !is_open;
        }

        let popup_width = element
            .attributes
            .get_custom("popup-width")
            .or_else(|| element.attributes.get_custom("popup_width"))
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(rect.width().max(120.0));
        let popup_row_height = element
            .attributes
            .get_custom("popup-row-height")
            .or_else(|| element.attributes.get_custom("popup_row_height"))
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(26.0);
        let popup_padding = element
            .attributes
            .get_custom("popup-padding")
            .or_else(|| element.attributes.get_custom("popup_padding"))
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(2.0);
        let popup_bg = element
            .attributes
            .get_custom("popup-background")
            .or_else(|| element.attributes.get_custom("popup_background"))
            .and_then(|c| Self::parse_color_static(c))
            .unwrap_or(Color32::from_rgba_premultiplied(63, 77, 91, 245));
        let popup_selected_bg = element
            .attributes
            .get_custom("popup-selected-background")
            .or_else(|| element.attributes.get_custom("popup_selected_background"))
            .and_then(|c| Self::parse_color_static(c))
            .unwrap_or(Color32::from_rgba_premultiplied(79, 92, 106, 255));
        let popup_border = element
            .attributes
            .get_custom("popup-border-color")
            .or_else(|| element.attributes.get_custom("popup_border_color"))
            .and_then(|c| Self::parse_color_static(c))
            .unwrap_or(Color32::from_rgba_premultiplied(255, 255, 255, 32));
        let popup_check_color = element
            .attributes
            .get_custom("popup-check-color")
            .or_else(|| element.attributes.get_custom("popup_check_color"))
            .and_then(|c| Self::parse_color_static(c))
            .unwrap_or(Color32::WHITE);
        let popup_height = popup_padding * 2.0 + popup_row_height * options.len() as f32;
        let popup_rect = Rect::from_min_size(
            Pos2::new(rect.min.x, rect.max.y + 4.0),
            Vec2::new(popup_width, popup_height),
        );

        if new_open_state && !options.is_empty() {
            ui.painter()
                .rect_filled(popup_rect, CornerRadius::same(8), popup_bg);
            ui.painter().rect_stroke(
                popup_rect,
                CornerRadius::same(8),
                egui::Stroke::new(1.0, popup_border),
                egui::StrokeKind::Inside,
            );

            for (index, (value, text)) in options.iter().enumerate() {
                let option_rect = Rect::from_min_max(
                    Pos2::new(
                        popup_rect.min.x + popup_padding,
                        popup_rect.min.y + popup_padding + popup_row_height * index as f32,
                    ),
                    Pos2::new(
                        popup_rect.max.x - popup_padding,
                        popup_rect.min.y + popup_padding + popup_row_height * (index as f32 + 1.0),
                    ),
                );
                let option_id = egui::Id::new(format!("select_{}_{}", id, value));
                let option_response = ui.interact(option_rect, option_id, egui::Sense::click());
                let is_selected = value == &selected;
                if is_selected || option_response.hovered() {
                    ui.painter().rect_filled(
                        option_rect,
                        CornerRadius::same(8),
                        if is_selected {
                            popup_selected_bg
                        } else {
                            Color32::from_white_alpha(14)
                        },
                    );
                }
                let option_color = if is_selected {
                    Color32::WHITE
                } else {
                    text_color
                };
                let option_galley = self.layout_button_text_galley(
                    ui,
                    text.clone(),
                    egui::FontId::proportional(font_size),
                    option_color,
                    option_rect.width() - 32.0,
                    false,
                );
                let option_pos = Pos2::new(
                    option_rect.min.x + 14.0,
                    option_rect.center().y - (option_galley.size().y / 2.0),
                );
                ui.painter().galley(option_pos, option_galley, option_color);
                if is_selected {
                    let check_rect = Rect::from_center_size(
                        Pos2::new(option_rect.max.x - 16.0, option_rect.center().y),
                        Vec2::new(12.0, 12.0),
                    );
                    Self::paint_checkmark_in_rect(ui, check_rect, popup_check_color);
                }
                if option_response.clicked() {
                    new_selected = value.clone();
                    new_open_state = false;
                }
            }

            let pointer_pos = ui.ctx().input(|i| i.pointer.interact_pos());
            let pointer_pressed = ui.ctx().input(|i| i.pointer.any_pressed());
            if pointer_pressed
                && !response.hovered()
                && pointer_pos
                    .map(|pos| !popup_rect.contains(pos) && !rect.contains(pos))
                    .unwrap_or(false)
            {
                new_open_state = false;
            }
        }

        self.interaction_state
            .select_open_states
            .insert(id.to_string(), new_open_state);

        // If selection changed, record it
        if new_selected != selected {
            self.interaction_state
                .text_inputs
                .insert(id.to_string(), new_selected.clone());
            self.interaction_state
                .select_open_states
                .insert(id.to_string(), false);
            result
                .select_changes
                .insert(id.to_string(), new_selected.clone());

            // Also record as button action if action attribute is set
            if !action.is_empty() {
                let full_action = format!("{}:{}", action, new_selected);
                result.button_clicks.insert(id.to_string(), true);
                result.button_actions.insert(id.to_string(), full_action);
            }
        }
    }

    /// 渲染单个元素
    fn render_element(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        use once_cell::sync::Lazy;
        use std::sync::Mutex;
        static RENDER_LOGGED: Lazy<Mutex<std::collections::HashSet<String>>> =
            Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
        let mut logged = RENDER_LOGGED.lock().unwrap();
        let element_type_str = format!("{:?}", element.element_type);
        let element_id = element
            .attributes
            .id
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("unnamed");
        let log_key = format!("{}:{}", element_type_str, element_id);

        // 检查绝对定位属性
        let has_float = element.attributes.get_custom("float").is_some();
        let has_pos = element.attributes.get_custom("pos").is_some();
        let is_absolute = element
            .attributes
            .get_custom("is_absolute")
            .map(|s| s == "true")
            .unwrap_or(false);

        if !logged.contains(&log_key) {
            eprintln!(
                "[渲染] 渲染元素: {} (id: {}, 子元素数: {}, float={}, pos={}, is_absolute={})",
                element_type_str,
                element_id,
                element.children.len(),
                has_float,
                has_pos,
                is_absolute
            );
            if has_pos {
                if let Some(pos_str) = element.attributes.get_custom("pos") {
                    eprintln!("[渲染]   pos 值: {}", pos_str);
                }
            }
            logged.insert(log_key);
        }
        drop(logged);

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
            ElementType::Select => {} // Select only rendered in Taffy path
            ElementType::Overlay => self.render_overlay(ui, element, result),
        }
    }

    /// 渲染页面
    fn render_page(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        // 移除频繁的日志输出，避免每帧都输出

        // 获取窗口尺寸（从 XML 或使用 DpiConfig 的值，确保与窗口大小一致）
        let page_width = element
            .attributes
            .width
            .unwrap_or(self.dpi_config.window_width);
        let page_height = element
            .attributes
            .height
            .unwrap_or(self.dpi_config.window_height);

        // 先分配整个页面的矩形空间
        let (full_rect, _response) =
            ui.allocate_exact_size(egui::vec2(page_width, page_height), egui::Sense::hover());

        // 应用圆角裁剪（窗口圆角）
        // 注意：圆角效果通过背景图片和窗口透明背景实现
        // 实际的窗口圆角由 Windows 系统处理（通过 DwmSetWindowAttribute）

        // 如果有背景图片，在底层渲染背景（填充整个窗口，而不是只填充页面矩形）
        if let Some(background) = &element.attributes.background {
            // 记录背景路径和窗口大小（只在首次加载时输出）
            use once_cell::sync::Lazy;
            use std::sync::Mutex;
            static BACKGROUND_LOADED: Lazy<Mutex<std::collections::HashSet<String>>> =
                Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
            let mut loaded = BACKGROUND_LOADED.lock().unwrap();
            if !loaded.contains(background) {
                eprintln!("[背景] 加载背景图片: {}", background);
                eprintln!(
                    "[背景]   use_2x: {}, 窗口大小: {}x{}",
                    self.dpi_config.use_2x,
                    self.dpi_config.window_width,
                    self.dpi_config.window_height
                );
                loaded.insert(background.clone());
            }
            drop(loaded);

            if let Some(texture) =
                self.resource_cache
                    .get_background(ui.ctx(), &self.dpi_config, background)
            {
                let texture_size = texture.size();
                let window_rect = ui.max_rect();

                // 只在首次成功加载时输出
                static BACKGROUND_SUCCESS: Lazy<Mutex<std::collections::HashSet<String>>> =
                    Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
                let mut success = BACKGROUND_SUCCESS.lock().unwrap();
                if !success.contains(background) {
                    eprintln!(
                        "[背景] ✓ 背景图片加载成功: {} ({}x{}), 窗口: {}x{}",
                        background,
                        texture_size[0],
                        texture_size[1],
                        window_rect.width(),
                        window_rect.height()
                    );

                    // 检查图片尺寸和窗口尺寸是否匹配
                    let width_match = (texture_size[0] as f32 - window_rect.width()).abs() < 1.0;
                    let height_match = (texture_size[1] as f32 - window_rect.height()).abs() < 1.0;
                    if !width_match || !height_match {
                        eprintln!("[背景] ⚠️  警告: 背景图片尺寸与窗口大小不匹配！");
                        eprintln!(
                            "[背景]   图片: {}x{}, 窗口: {}x{}, 差异: {}x{}",
                            texture_size[0],
                            texture_size[1],
                            window_rect.width(),
                            window_rect.height(),
                            (texture_size[0] as f32 - window_rect.width()).abs(),
                            (texture_size[1] as f32 - window_rect.height()).abs()
                        );
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
                static BACKGROUND_FAILED: Lazy<Mutex<std::collections::HashSet<String>>> =
                    Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
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
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );

        // 先渲染绝对定位的元素（如关闭按钮），它们不参与布局流
        let mut absolute_children = Vec::new();
        let mut layout_children = Vec::new();

        for child in &element.children {
            // 检查是否有绝对定位（position 属性或 float="true" + pos 属性）
            let is_absolute = child.attributes.get_custom("position").is_some()
                || child
                    .attributes
                    .get_custom("is_absolute")
                    .map(|s| s == "true")
                    .unwrap_or(false)
                || (child
                    .attributes
                    .get_custom("float")
                    .map(|s| s == "true")
                    .unwrap_or(false)
                    && child.attributes.get_custom("pos").is_some());

            if is_absolute {
                absolute_children.push(child);
            } else {
                layout_children.push(child);
            }
        }

        // 先渲染绝对定位的元素（直接使用 painter，不参与布局流）
        // 注意：绝对定位的元素需要使用父级 UI 来获取正确的窗口坐标
        // 但是需要传递 full_rect 信息，以便正确计算坐标
        for child in &absolute_children {
            // 对于绝对定位的元素，使用父级 UI 而不是 page_ui，以确保坐标系统正确
            // 同时需要传递 full_rect 信息，以便在 render_button 中正确计算坐标
            // 创建一个临时的上下文来传递 full_rect 信息
            // 由于 render_element 不接受额外的参数，我们需要在 render_button 中使用 viewport_rect
            // 但 viewport_rect 可能返回的是相对于整个屏幕的坐标，而不是相对于窗口的
            // 所以我们需要使用 full_rect.min 作为原点
            self.render_element(ui, child, result);
        }

        // 然后渲染布局流中的元素
        // 如果第一个子元素是布局容器（VBox/HBox），直接使用 page_ui 渲染，让它使用正常的布局流
        // 否则使用绝对定位渲染
        if let Some(first_child) = layout_children.first() {
            match &first_child.element_type {
                ElementType::VBox | ElementType::HBox => {
                    // 布局容器直接使用 page_ui 渲染，使用正常的布局流
                    // render_vbox/render_hbox 会自己处理空间分配，所以直接调用即可
                    self.render_element(&mut page_ui, first_child, result);
                }
                _ => {
                    // 其他元素使用绝对定位渲染
                    for child in &layout_children {
                        current_y = self.render_element_at_y_absolute(
                            &mut page_ui,
                            child,
                            current_y,
                            full_rect,
                            result,
                        );
                    }
                }
            }
        }
    }

    /// 在绝对 Y 坐标渲染元素（用于 Page 内的绝对定位）
    fn render_element_at_y_absolute(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        y: f32,
        parent_rect: egui::Rect,
        result: &mut RenderResult,
    ) -> f32 {
        match &element.element_type {
            ElementType::VBox => {
                // VBox: 递归渲染子元素
                let mut current_y = y;
                for child in &element.children {
                    current_y = self.render_element_at_y_absolute(
                        ui,
                        child,
                        current_y,
                        parent_rect,
                        result,
                    );
                }
                current_y
            }
            ElementType::Spacer => {
                // Spacer: 只增加 Y 坐标，不渲染任何东西
                let height = element.attributes.height.unwrap_or(0.0);
                // 移除频繁的日志输出
                y + height
            }
            ElementType::HBox
            | ElementType::Image
            | ElementType::Button
            | ElementType::Checkbox
            | ElementType::Label => {
                // 获取元素高度
                let height = element.attributes.height.unwrap_or(40.0);

                // 创建一个从当前 Y 位置开始的矩形
                let element_rect = egui::Rect::from_min_size(
                    egui::pos2(parent_rect.min.x, y),
                    egui::vec2(parent_rect.width(), height),
                );

                // 创建子 UI，直接使用 painter 在指定矩形内绘制
                let mut child_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(element_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
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
    fn render_element_at_y(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        y: f32,
        result: &mut RenderResult,
    ) -> f32 {
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
            ElementType::HBox
            | ElementType::Image
            | ElementType::Button
            | ElementType::Checkbox
            | ElementType::Label => {
                // 获取元素高度
                let height = element.attributes.height.unwrap_or(40.0);

                // 在当前 Y 位置渲染元素
                let rect = egui::Rect::from_min_size(
                    egui::pos2(0.0, y),
                    egui::vec2(ui.available_width(), height),
                );

                let mut child_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
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
        let halign = element
            .attributes
            .get_custom("halign")
            .map(|s| s.as_str())
            .unwrap_or("left");
        // NSIS 格式：valign 用于垂直对齐（在 VBox 中，align 是水平对齐，valign 是垂直对齐）
        let valign = element
            .attributes
            .get_custom("valign")
            .map(|s| s.as_str())
            .unwrap_or("top");

        let available_height = ui.available_height();
        let available_width = ui.available_width();

        // 获取 VBox 的尺寸（如果有指定）
        // 如果没有指定 width/height，使用全部可用空间（而不是 0）
        let vbox_width = element.attributes.width.unwrap_or(available_width);
        let vbox_height = element.attributes.height.unwrap_or(available_height);

        // 添加调试日志
        static VBOX_SIZE_LOGGED: Lazy<Mutex<std::collections::HashSet<String>>> =
            Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
        let mut logged = VBOX_SIZE_LOGGED.lock().unwrap();
        let vbox_id = element
            .attributes
            .id
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("unnamed");
        let log_key = format!("{}:{}x{}", vbox_id, vbox_width, vbox_height);
        if !logged.contains(&log_key) {
            eprintln!(
                "[VBox尺寸] VBox '{}': 指定尺寸={}x{}, 可用空间={}x{}, 最终尺寸={}x{}",
                vbox_id,
                element.attributes.width.unwrap_or(0.0),
                element.attributes.height.unwrap_or(0.0),
                available_width,
                available_height,
                vbox_width,
                vbox_height
            );
            logged.insert(log_key);
        }
        drop(logged);

        // 分配 VBox 的矩形空间
        let (vbox_rect, _) =
            ui.allocate_exact_size(egui::vec2(vbox_width, vbox_height), egui::Sense::hover());

        // 如果有背景图片或背景颜色，先渲染背景
        // 注意：在 egui 中，painter 的绘制是在当前层的，需要确保背景在最底层
        // 如果有背景图片，优先使用背景图片；如果有背景颜色但没有背景图片，使用背景颜色
        let mut has_bg_image = false;
        let mut bg_image_path = None;
        let mut bg_color = None;

        if let Some(background) = &element.attributes.background {
            if background.starts_with("assets/")
                || background.ends_with(".png")
                || background.ends_with(".jpg")
                || background.contains("@2x")
            {
                // 背景图片
                bg_image_path = Some(background.clone());
            } else {
                // 背景颜色（bkcolor）
                bg_color = self.parse_color(background);
            }
        }

        // 先渲染背景图片（如果有）
        if let Some(background) = bg_image_path {
            if ui.is_rect_visible(vbox_rect) {
                use once_cell::sync::Lazy;
                use std::sync::Mutex;
                static VBOX_BG_LOADED: Lazy<Mutex<std::collections::HashSet<String>>> =
                    Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
                let mut loaded = VBOX_BG_LOADED.lock().unwrap();
                if !loaded.contains(&background) {
                    eprintln!("[VBox背景] 尝试加载背景图片: {}", background);
                    eprintln!(
                        "[VBox背景]   VBox尺寸: {}x{}, 矩形: {:?}",
                        vbox_width, vbox_height, vbox_rect
                    );
                    loaded.insert(background.clone());
                }
                drop(loaded);

                let corner_radius = self.get_corner_radius(element);
                let stroke = self.get_border_stroke(element);
                if let Some(texture) =
                    self.resource_cache
                        .get_background(ui.ctx(), &self.dpi_config, &background)
                {
                    let texture_size = texture.size();
                    static VBOX_BG_SUCCESS: Lazy<Mutex<std::collections::HashSet<String>>> =
                        Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
                    let mut success = VBOX_BG_SUCCESS.lock().unwrap();
                    if !success.contains(&background) {
                        eprintln!(
                            "[VBox背景] ✓ 背景图片加载成功: {} ({}x{}), VBox: {}x{}",
                            background, texture_size[0], texture_size[1], vbox_width, vbox_height
                        );
                        success.insert(background.clone());
                    }
                    drop(success);

                    has_bg_image = true;

                    if let Some(stroke) = stroke {
                        // 有边框：先绘制背景图片，再绘制边框
                        ui.painter().image(
                            texture.id(),
                            vbox_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                        ui.painter().rect_stroke(
                            vbox_rect,
                            corner_radius,
                            stroke,
                            egui::epaint::StrokeKind::Outside,
                        );
                    } else {
                        // 无边框：直接绘制背景图片
                        ui.painter().image(
                            texture.id(),
                            vbox_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                } else {
                    static VBOX_BG_FAILED: Lazy<Mutex<std::collections::HashSet<String>>> =
                        Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
                    let mut failed = VBOX_BG_FAILED.lock().unwrap();
                    if !failed.contains(&background) {
                        eprintln!("[VBox背景] ✗ 背景图片加载失败: {}", background);
                        failed.insert(background.clone());
                    }
                }
            }
        }

        // 如果没有背景图片，但有背景颜色，渲染背景颜色
        if !has_bg_image {
            if let Some(color) = bg_color {
                if ui.is_rect_visible(vbox_rect) {
                    let corner_radius = self.get_corner_radius(element);
                    let stroke = self.get_border_stroke(element);

                    ui.painter().rect_filled(vbox_rect, corner_radius, color);
                    if let Some(stroke) = stroke {
                        ui.painter().rect_stroke(
                            vbox_rect,
                            corner_radius,
                            stroke,
                            egui::epaint::StrokeKind::Outside,
                        );
                    }
                }
            }
        }

        // 检查是否有 flex 子元素
        let has_flex_children = element
            .children
            .iter()
            .any(|child| child.attributes.flex.is_some());

        // 在 VBox 矩形内创建子 UI
        let mut vbox_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(vbox_rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );

        // 添加调试日志
        use once_cell::sync::Lazy;
        use std::sync::Mutex;
        static VBOX_RENDER_LOGGED: Lazy<Mutex<std::collections::HashSet<String>>> =
            Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
        let mut logged = VBOX_RENDER_LOGGED.lock().unwrap();
        let vbox_id = element
            .attributes
            .id
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("unnamed");
        let log_key = format!(
            "{}:{}:{}:{}",
            vbox_id,
            has_flex_children,
            align,
            element.children.len()
        );
        if !logged.contains(&log_key) {
            eprintln!("[VBox渲染] VBox '{}' 渲染路径: has_flex={}, align={}, halign={}, valign={}, 子元素数={}", 
                vbox_id, has_flex_children, align, halign, valign, element.children.len());
            // 列出所有子元素的 ID
            for (idx, child) in element.children.iter().enumerate() {
                let child_id = child
                    .attributes
                    .id
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("unnamed");
                let child_type = format!("{:?}", child.element_type);
                eprintln!(
                    "[VBox渲染]   子元素 [{}]: {} (id: {})",
                    idx, child_type, child_id
                );
            }
            logged.insert(log_key);
        }
        drop(logged);

        // 添加调试日志：显示第一个 VBox 的渲染分支选择
        static VBOX_BRANCH_LOGGED: Lazy<Mutex<std::collections::HashSet<String>>> =
            Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
        let mut branch_logged = VBOX_BRANCH_LOGGED.lock().unwrap();
        let vbox_id = element
            .attributes
            .id
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("unnamed");
        let branch_key = format!(
            "{}:{}:{}:{}:{}",
            vbox_id, has_flex_children, align, halign, valign
        );
        if !branch_logged.contains(&branch_key) {
            eprintln!(
                "[VBox分支] VBox '{}' 选择渲染分支: has_flex={}, align={}, halign={}, valign={}",
                vbox_id, has_flex_children, align, halign, valign
            );
            branch_logged.insert(branch_key);
        }
        drop(branch_logged);

        if has_flex_children {
            self.render_vbox_with_flex(&mut vbox_ui, element, result);
        } else if align != "left" || halign != "left" || valign != "top" {
            self.render_vbox_with_align(&mut vbox_ui, element, result, align, halign, valign);
        } else {
            // 在闭包外部先分离绝对定位和布局流子元素
            let mut absolute_children = Vec::new();
            let mut layout_children = Vec::new();

            for child in &element.children {
                // 检查是否有绝对定位（position 属性或 float="true" + pos 属性）
                let is_absolute = child.attributes.get_custom("position").is_some()
                    || child
                        .attributes
                        .get_custom("is_absolute")
                        .map(|s| s == "true")
                        .unwrap_or(false)
                    || (child
                        .attributes
                        .get_custom("float")
                        .map(|s| s == "true")
                        .unwrap_or(false)
                        && child.attributes.get_custom("pos").is_some());

                if is_absolute {
                    absolute_children.push(child);
                } else {
                    layout_children.push(child);
                }
            }

            // 使用 allocate_ui_with_layout 确保占据全部可用高度
            vbox_ui.allocate_ui_with_layout(
                egui::vec2(vbox_rect.width(), vbox_rect.height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, spacing);

                    if let Some((top, _right, _bottom, _left)) = padding {
                        ui.add_space(top);
                    }

                    static VBOX_CHILDREN_LOGGED: Lazy<Mutex<std::collections::HashSet<String>>> = Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
                    let mut logged = VBOX_CHILDREN_LOGGED.lock().unwrap();
                    let vbox_id = element.attributes.id.as_ref().map(|s| s.as_str()).unwrap_or("unnamed");
                    let log_key = format!("{}:{}", vbox_id, element.children.len());
                    if !logged.contains(&log_key) {
                        eprintln!("[VBox子元素] VBox '{}' 开始渲染 {} 个子元素 (布局流: {}, 绝对定位: {})", 
                            vbox_id, element.children.len(), layout_children.len(), absolute_children.len());
                        for (idx, child) in layout_children.iter().enumerate() {
                            let child_type = format!("{:?}", child.element_type);
                            let child_id = child.attributes.id.as_ref().map(|s| s.as_str()).unwrap_or("unnamed");
                            eprintln!("[VBox子元素]   [布局流 {}] {} (id: {})", idx, child_type, child_id);
                        }
                        for (idx, child) in absolute_children.iter().enumerate() {
                            let child_type = format!("{:?}", child.element_type);
                            let child_id = child.attributes.id.as_ref().map(|s| s.as_str()).unwrap_or("unnamed");
                            eprintln!("[VBox子元素]   [绝对定位 {}] {} (id: {})", idx, child_type, child_id);
                        }
                        logged.insert(log_key);
                    }
                    drop(logged);

                    // 只渲染布局流中的元素
                    for child in &layout_children {
                        self.render_element(ui, child, result);
                    }

                    if let Some((_top, _right, bottom, _left)) = padding {
                        ui.add_space(bottom);
                    }
                }
            );

            // 在闭包外部渲染绝对定位的元素（使用父级 UI，确保坐标正确）
            // 注意：绝对定位的元素需要使用父级 UI 来获取正确的窗口坐标
            // 在 VBox 内部，绝对定位的元素应该相对于窗口（Page），而不是 VBox
            for child in &absolute_children {
                self.render_element(ui, child, result);
            }
        }
    }

    /// 渲染带对齐的垂直布局
    fn render_vbox_with_align(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        result: &mut RenderResult,
        _align: &str,
        halign: &str,
        valign: &str,
    ) {
        let spacing = element.attributes.spacing.unwrap_or(0.0);
        let padding = element.attributes.padding;

        // 在闭包外部先分离绝对定位和布局流子元素
        let mut absolute_children = Vec::new();
        let mut layout_children = Vec::new();

        for child in &element.children {
            // 检查是否有绝对定位（position 属性或 float="true" + pos 属性）
            let is_absolute = child.attributes.get_custom("position").is_some()
                || child
                    .attributes
                    .get_custom("is_absolute")
                    .map(|s| s == "true")
                    .unwrap_or(false)
                || (child
                    .attributes
                    .get_custom("float")
                    .map(|s| s == "true")
                    .unwrap_or(false)
                    && child.attributes.get_custom("pos").is_some());

            if is_absolute {
                absolute_children.push(child);
            } else {
                layout_children.push(child);
            }
        }

        // 添加调试日志
        static VBOX_ALIGN_CHILDREN_LOGGED: Lazy<Mutex<std::collections::HashSet<String>>> =
            Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
        let mut logged = VBOX_ALIGN_CHILDREN_LOGGED.lock().unwrap();
        let vbox_id = element
            .attributes
            .id
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("unnamed");
        let log_key = format!("{}:{}", vbox_id, element.children.len());
        if !logged.contains(&log_key) {
            eprintln!("[VBox子元素] VBox '{}' (render_vbox_with_align) 开始渲染 {} 个子元素 (布局流: {}, 绝对定位: {})", 
                vbox_id, element.children.len(), layout_children.len(), absolute_children.len());
            for (idx, child) in layout_children.iter().enumerate() {
                let child_type = format!("{:?}", child.element_type);
                let child_id = child
                    .attributes
                    .id
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("unnamed");
                eprintln!(
                    "[VBox子元素]   [布局流 {}] {} (id: {})",
                    idx, child_type, child_id
                );
            }
            for (idx, child) in absolute_children.iter().enumerate() {
                let child_type = format!("{:?}", child.element_type);
                let child_id = child
                    .attributes
                    .id
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("unnamed");
                eprintln!(
                    "[VBox子元素]   [绝对定位 {}] {} (id: {})",
                    idx, child_type, child_id
                );
            }
            logged.insert(log_key);
        }
        drop(logged);

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, spacing);

            if let Some((top, _right, _bottom, _left)) = padding {
                ui.add_space(top);
            }

            // 计算布局流子元素总高度
            let mut total_height = 0.0;
            for child in &layout_children {
                if let Some(height) = child.attributes.height {
                    total_height += height;
                }
            }
            total_height += spacing * (layout_children.len() as f32 - 1.0).max(0.0);

            let available_height = ui.available_height();
            let remaining_height = (available_height - total_height).max(0.0);

            // 垂直对齐（NSIS 格式：valign 用于垂直对齐，align 用于水平对齐）
            match valign {
                "center" | "vcenter" | "middle" => {
                    ui.add_space(remaining_height / 2.0);
                }
                "bottom" => {
                    ui.add_space(remaining_height);
                }
                "space-between" => {
                    let gap = if layout_children.len() > 1 {
                        remaining_height / (layout_children.len() as f32 - 1.0)
                    } else {
                        0.0
                    };

                    for (i, child) in layout_children.iter().enumerate() {
                        self.render_element_with_halign(ui, child, result, halign);
                        if i < layout_children.len() - 1 {
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

            // 渲染布局流子元素（带水平对齐）
            for child in &layout_children {
                self.render_element_with_halign(ui, child, result, halign);
            }

            if let Some((_top, _right, bottom, _left)) = padding {
                ui.add_space(bottom);
            }
        });

        // 在闭包外部渲染绝对定位的元素（使用父级 UI，确保坐标正确）
        // 注意：绝对定位的元素需要使用父级 UI 来获取正确的窗口坐标
        // 在 VBox 内部，绝对定位的元素应该相对于窗口（Page），而不是 VBox
        for child in &absolute_children {
            self.render_element(ui, child, result);
        }
    }

    /// 渲染元素（带水平对齐）
    fn render_element_with_halign(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        result: &mut RenderResult,
        halign: &str,
    ) {
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
    fn render_vbox_with_flex(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        result: &mut RenderResult,
    ) {
        let spacing = element.attributes.spacing.unwrap_or(0.0);

        // 在闭包外部先分离绝对定位和布局流子元素
        let mut absolute_children = Vec::new();
        let mut layout_children = Vec::new();

        for child in &element.children {
            // 检查是否有绝对定位（position 属性或 float="true" + pos 属性）
            let is_absolute = child.attributes.get_custom("position").is_some()
                || child
                    .attributes
                    .get_custom("is_absolute")
                    .map(|s| s == "true")
                    .unwrap_or(false)
                || (child
                    .attributes
                    .get_custom("float")
                    .map(|s| s == "true")
                    .unwrap_or(false)
                    && child.attributes.get_custom("pos").is_some());

            if is_absolute {
                absolute_children.push(child);
            } else {
                layout_children.push(child);
            }
        }

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, spacing);

            // 第一遍：计算固定高度元素和总 flex 权重
            let mut fixed_height = 0.0;
            let mut total_flex = 0.0;

            for child in &layout_children {
                if let Some(flex) = child.attributes.flex {
                    total_flex += flex;
                } else if let Some(height) = child.attributes.height {
                    fixed_height += height;
                }
            }

            // 计算剩余可用空间
            let available_height = ui.available_height();
            let spacing_total = spacing * (layout_children.len() as f32 - 1.0).max(0.0);
            let remaining_height = (available_height - fixed_height - spacing_total).max(0.0);

            // 第二遍：渲染布局流元素
            for child in &layout_children {
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
        let valign = element
            .attributes
            .get_custom("valign")
            .map(|s| s.as_str())
            .unwrap_or("top");
        let height = element.attributes.height;

        // 在闭包外部先分离绝对定位和布局流子元素
        let mut absolute_children = Vec::new();
        let mut layout_children = Vec::new();

        for child in &element.children {
            // 检查是否有绝对定位（position 属性或 float="true" + pos 属性）
            let is_absolute = child.attributes.get_custom("position").is_some()
                || child
                    .attributes
                    .get_custom("is_absolute")
                    .map(|s| s == "true")
                    .unwrap_or(false)
                || (child
                    .attributes
                    .get_custom("float")
                    .map(|s| s == "true")
                    .unwrap_or(false)
                    && child.attributes.get_custom("pos").is_some());

            if is_absolute {
                absolute_children.push(child);
            } else {
                layout_children.push(child);
            }
        }

        // 添加调试日志
        static HBOX_CHILDREN_LOGGED: Lazy<Mutex<std::collections::HashSet<String>>> =
            Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
        let mut logged = HBOX_CHILDREN_LOGGED.lock().unwrap();
        let hbox_id = element
            .attributes
            .id
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("unnamed");
        let log_key = format!("{}:{}", hbox_id, element.children.len());
        if !logged.contains(&log_key) {
            eprintln!(
                "[HBox子元素] HBox '{}' 开始渲染 {} 个子元素 (布局流: {}, 绝对定位: {})",
                hbox_id,
                element.children.len(),
                layout_children.len(),
                absolute_children.len()
            );
            for (idx, child) in layout_children.iter().enumerate() {
                let child_type = format!("{:?}", child.element_type);
                let child_id = child
                    .attributes
                    .id
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("unnamed");
                eprintln!(
                    "[HBox子元素]   [布局流 {}] {} (id: {})",
                    idx, child_type, child_id
                );
            }
            for (idx, child) in absolute_children.iter().enumerate() {
                let child_type = format!("{:?}", child.element_type);
                let child_id = child
                    .attributes
                    .id
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("unnamed");
                eprintln!(
                    "[HBox子元素]   [绝对定位 {}] {} (id: {})",
                    idx, child_type, child_id
                );
            }
            logged.insert(log_key);
        }
        drop(logged);

        // 检查是否有 flex 子元素（只检查布局流子元素）
        let has_flex_children = layout_children
            .iter()
            .any(|child| child.attributes.flex.is_some());

        // 如果指定了高度，先分配固定高度的区域
        if let Some(h) = height {
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), h),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(spacing, 0.0);

                    if let Some((_top, _right, _bottom, left)) = padding {
                        ui.add_space(left);
                    }

                    if has_flex_children {
                        // 传递 layout_children 而不是 element.children
                        self.render_hbox_flex_children_list(ui, &layout_children, spacing, result);
                    } else {
                        // 只渲染布局流子元素
                        for child in &layout_children {
                            self.render_element(ui, child, result);
                        }
                    }

                    if let Some((_top, right, _bottom, _left)) = padding {
                        ui.add_space(right);
                    }
                },
            );
            // 移除频繁的日志输出
        } else if has_flex_children {
            self.render_hbox_with_flex_list(ui, &layout_children, spacing, result);
        } else if align != "left" || valign != "top" {
            self.render_hbox_with_align_list(
                ui,
                &layout_children,
                spacing,
                padding,
                align,
                valign,
                result,
            );
        } else {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(spacing, 0.0);

                if let Some((_top, _right, _bottom, left)) = padding {
                    ui.add_space(left);
                }

                // 只渲染布局流子元素
                for child in &layout_children {
                    self.render_element(ui, child, result);
                }

                if let Some((_top, right, _bottom, _left)) = padding {
                    ui.add_space(right);
                }
            });
        }

        // 在闭包外部渲染绝对定位的元素（使用父级 UI，确保坐标正确）
        // 注意：绝对定位的元素需要使用父级 UI 来获取正确的窗口坐标
        // 在 HBox 内部，绝对定位的元素应该相对于窗口（Page），而不是 HBox
        for child in &absolute_children {
            self.render_element(ui, child, result);
        }
    }

    /// 渲染 HBox 的 flex 子元素（不包含外层 horizontal）
    fn render_hbox_flex_children(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        result: &mut RenderResult,
    ) {
        let spacing = element.attributes.spacing.unwrap_or(0.0);
        let children_refs: Vec<&LayoutElement> = element.children.iter().collect();
        self.render_hbox_flex_children_list(ui, &children_refs, spacing, result);
    }

    /// 渲染 HBox 的 flex 子元素列表（不包含外层 horizontal）
    fn render_hbox_flex_children_list(
        &mut self,
        ui: &mut Ui,
        children: &[&LayoutElement],
        spacing: f32,
        result: &mut RenderResult,
    ) {
        // 第一遍：计算固定宽度元素和总 flex 权重
        let mut fixed_width = 0.0;
        let mut total_flex = 0.0;

        for child in children {
            if let Some(flex) = child.attributes.flex {
                total_flex += flex;
            } else if let Some(width) = child.attributes.width {
                fixed_width += width;
            }
        }

        // 计算剩余可用空间
        let available_width = ui.available_width();
        let spacing_total = spacing * (children.len() as f32 - 1.0).max(0.0);
        let remaining_width = (available_width - fixed_width - spacing_total).max(0.0);

        // 第二遍：渲染元素
        for child in children {
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
    fn render_hbox_with_align(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        result: &mut RenderResult,
        align: &str,
        valign: &str,
    ) {
        let spacing = element.attributes.spacing.unwrap_or(0.0);
        let padding = element.attributes.padding;
        let children_refs: Vec<&LayoutElement> = element.children.iter().collect();
        self.render_hbox_with_align_list(
            ui,
            &children_refs,
            spacing,
            padding,
            align,
            valign,
            result,
        );
    }

    /// 渲染带对齐的水平布局（使用子元素列表）
    fn render_hbox_with_align_list(
        &mut self,
        ui: &mut Ui,
        children: &[&LayoutElement],
        spacing: f32,
        padding: Option<(f32, f32, f32, f32)>,
        align: &str,
        valign: &str,
        result: &mut RenderResult,
    ) {
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
            for child in children {
                if let Some(width) = child.attributes.width {
                    total_width += width;
                } else if let Some(min_width) = child.attributes.min_width {
                    total_width += min_width;
                }
            }
            total_width += spacing * (children.len() as f32 - 1.0).max(0.0);

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
                    let gap = if children.len() > 1 {
                        remaining_width / (children.len() as f32 - 1.0)
                    } else {
                        0.0
                    };

                    for (i, child) in children.iter().enumerate() {
                        self.render_element_with_valign(ui, child, result, vertical_align);
                        if i < children.len() - 1 {
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
            for child in children {
                self.render_element_with_valign(ui, child, result, vertical_align);
            }

            if let Some((_top, right, _bottom, _left)) = padding {
                ui.add_space(right);
            }
        });
    }

    /// 渲染元素（带垂直对齐）
    fn render_element_with_valign(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        result: &mut RenderResult,
        valign: egui::Align,
    ) {
        ui.with_layout(egui::Layout::top_down(valign), |ui| {
            self.render_element(ui, element, result);
        });
    }

    /// 渲染带 flex 的水平布局
    fn render_hbox_with_flex(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        result: &mut RenderResult,
    ) {
        let spacing = element.attributes.spacing.unwrap_or(0.0);
        let children_refs: Vec<&LayoutElement> = element.children.iter().collect();
        self.render_hbox_with_flex_list(ui, &children_refs, spacing, result);
    }

    /// 渲染带 flex 的水平布局（使用子元素列表）
    fn render_hbox_with_flex_list(
        &mut self,
        ui: &mut Ui,
        children: &[&LayoutElement],
        spacing: f32,
        result: &mut RenderResult,
    ) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(spacing, 0.0);

            // 第一遍：计算固定宽度元素和总 flex 权重
            let mut fixed_width = 0.0;
            let mut total_flex = 0.0;

            for child in children {
                if let Some(flex) = child.attributes.flex {
                    total_flex += flex;
                } else if let Some(width) = child.attributes.width {
                    fixed_width += width;
                }
            }

            // 计算剩余可用空间
            let available_width = ui.available_width();
            let spacing_total = spacing * (children.len() as f32 - 1.0).max(0.0);
            let remaining_width = (available_width - fixed_width - spacing_total).max(0.0);

            // 第二遍：渲染元素
            for child in children {
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

    /// 渲染空白占位（Container 和 Control）
    fn render_spacer(&mut self, ui: &mut Ui, element: &LayoutElement, _result: &mut RenderResult) {
        let width = element.attributes.width.unwrap_or(0.0);
        let height = element.attributes.height.unwrap_or(0.0);
        let padding = element.attributes.padding.unwrap_or((0.0, 0.0, 0.0, 0.0));
        let (pad_top, pad_right, pad_bottom, pad_left) = padding;

        // 水平方向：使用 width 占位
        // 垂直方向：使用 height 占位
        // 空 Spacer：作为弹性空间（在 HBox 中填充剩余空间）
        let rect = if width > 0.0 {
            // 水平 Spacer（在 HBox 中）
            // 注意：如果同时指定了 height，则应使用显式的 height；
            // 只有在未指定 height 时，才使用可用高度填满父容器。
            let content_height = if height > 0.0 {
                height
            } else {
                ui.available_height().max(0.0)
            };
            // NSIS 中 Control/Container 的 padding 表示内容区域的内边距：
            // 总宽度 = left + content_width + right
            // 总高度 = top + content_height + bottom
            let total_width = width + pad_left + pad_right;
            let total_height = content_height + pad_top + pad_bottom;

            let (outer_rect, _) =
                ui.allocate_exact_size(egui::vec2(total_width, total_height), egui::Sense::hover());
            // 实际内容区域（例如 logo 图片）位于 padding 之后
            egui::Rect::from_min_size(
                egui::pos2(outer_rect.min.x + pad_left, outer_rect.min.y + pad_top),
                egui::vec2(width, content_height),
            )
        } else if height > 0.0 {
            // 垂直 Spacer（在 VBox 中）- 使用 allocate_exact_size 而不是 add_space
            let total_height = height + pad_top + pad_bottom;
            let (outer_rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), total_height),
                egui::Sense::hover(),
            );
            egui::Rect::from_min_size(
                egui::pos2(outer_rect.min.x + pad_left, outer_rect.min.y + pad_top),
                egui::vec2(outer_rect.width() - pad_left - pad_right, height),
            )
        } else {
            // 空 Spacer：弹性空间（填充所有剩余空间）
            ui.allocate_space(egui::vec2(ui.available_width(), 0.0));
            return;
        };

        // 支持 Container 和 Control 的背景图片（bkimage）
        if let Some(background) = &element.attributes.background {
            if ui.is_rect_visible(rect) {
                // 解析背景（可能是图片路径或颜色）
                if background.starts_with("assets/")
                    || background.ends_with(".png")
                    || background.ends_with(".jpg")
                {
                    // 背景图片
                    // 先获取圆角和边框配置，避免借用冲突
                    let corner_radius = self.get_corner_radius(element);
                    let stroke = self.get_border_stroke(element);
                    if let Some(texture) =
                        self.resource_cache
                            .get_background(ui.ctx(), &self.dpi_config, background)
                    {
                        if let Some(stroke) = stroke {
                            // 有边框：先绘制背景图片，再绘制边框
                            ui.painter().image(
                                texture.id(),
                                rect,
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                egui::Color32::WHITE,
                            );
                            ui.painter().rect_stroke(
                                rect,
                                corner_radius,
                                stroke,
                                egui::epaint::StrokeKind::Outside,
                            );
                        } else {
                            // 无边框：直接绘制背景图片
                            ui.painter().image(
                                texture.id(),
                                rect,
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                egui::Color32::WHITE,
                            );
                        }
                    }
                } else {
                    // 背景颜色
                    if let Some(color) = self.parse_color(background) {
                        let corner_radius = self.get_corner_radius(element);
                        let stroke = self.get_border_stroke(element);

                        ui.painter().rect_filled(rect, corner_radius, color);
                        if let Some(stroke) = stroke {
                            ui.painter().rect_stroke(
                                rect,
                                corner_radius,
                                stroke,
                                egui::epaint::StrokeKind::Outside,
                            );
                        }
                    }
                }
            }
        }
    }

    /// 渲染弹性空间
    fn render_flex(&mut self, ui: &mut Ui, _element: &LayoutElement, _result: &mut RenderResult) {
        ui.allocate_ui_with_layout(
            ui.available_size(),
            egui::Layout::left_to_right(Align::LEFT),
            |ui| {
                ui.allocate_ui(ui.available_size(), |_ui| {});
            },
        );
    }

    /// 渲染按钮
    fn render_button(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        let id = element
            .attributes
            .id
            .as_ref()
            .unwrap_or(&"".to_string())
            .clone();
        let text = self.get_display_text(&element.attributes);
        let enabled = element.attributes.enabled.unwrap_or(true);

        let width = element.attributes.width.unwrap_or(120.0);
        let height = element.attributes.height.unwrap_or(40.0);
        let padding = element.attributes.padding.unwrap_or((0.0, 0.0, 0.0, 0.0));
        let (pad_top, pad_right, pad_bottom, pad_left) = padding;

        // 添加调试日志
        static BUTTON_LOGGED: Lazy<Mutex<std::collections::HashSet<String>>> =
            Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
        let mut logged = BUTTON_LOGGED.lock().unwrap();
        if !logged.contains(&id) {
            let available_size = ui.available_size();
            let max_rect = ui.max_rect();
            eprintln!(
                "[Button渲染] Button '{}': 尺寸={}x{}, 可用空间={:?}, max_rect={:?}",
                id, width, height, available_size, max_rect
            );
            logged.insert(id.clone());
        }
        drop(logged);

        // 获取字体和颜色配置
        let font_id = self.get_font_id(element);

        // 检查是否有绝对定位（position 属性或 float="true" + pos 属性）
        let is_float = element
            .attributes
            .get_custom("float")
            .map(|s| s == "true")
            .unwrap_or(false);
        let has_pos = element.attributes.get_custom("pos").is_some();
        let (rect, response) = if let Some(position_str) = element.attributes.get_custom("position")
        {
            // 解析 position="x,y"
            let coords: Vec<f32> = position_str
                .split(',')
                .map(|s| s.trim().parse::<f32>().ok())
                .filter_map(|x| x)
                .collect();

            if coords.len() == 2 {
                // 绝对定位：相对于窗口左上角
                // 注意：对于关闭按钮，position 是相对于窗口的绝对坐标
                // 在 render_page 中，绝对定位的元素使用父级 UI 渲染
                // 父级 UI 的 max_rect() 应该返回窗口的矩形（从 (0,0) 开始）
                // 但是为了确保正确，我们使用 viewport_rect() 来获取窗口的实际坐标
                let viewport_rect = ui.ctx().viewport_rect();
                // position="x,y" 中的 x 和 y 是相对于窗口左上角的像素坐标
                // viewport_rect.min 是窗口在屏幕上的位置，我们需要相对于窗口的坐标
                // 所以直接使用 coords[0] 和 coords[1] 作为相对于窗口左上角的偏移
                let pos = egui::pos2(
                    viewport_rect.min.x + coords[0],
                    viewport_rect.min.y + coords[1],
                );
                let size = egui::vec2(width, height);
                let rect = egui::Rect::from_min_size(pos, size);

                // 对于绝对定位的元素，需要：
                // 1. 使用 painter 直接绘制（在布局流之外）
                // 2. 使用 interact 处理交互（在布局流之外）
                // 注意：绝对定位的元素不应该影响布局流，所以使用 painter 和 interact
                let response = ui.interact(rect, ui.id().with("abs_pos"), egui::Sense::click());
                (rect, response)
            } else if is_float && has_pos {
                // float="true" + pos="x1,y1,x2,y2" 格式的绝对定位
                if let Some(pos_str) = element.attributes.get_custom("pos") {
                    let coords: Vec<f32> = pos_str
                        .split(',')
                        .map(|s| s.trim().parse::<f32>().ok())
                        .filter_map(|x| x)
                        .collect();

                    if coords.len() == 4 {
                        let x1 = coords[0];
                        let y1 = coords[1];
                        let x2 = coords[2];
                        let y2 = coords[3];
                        let viewport_rect = ui.ctx().viewport_rect();
                        let pos = egui::pos2(viewport_rect.min.x + x1, viewport_rect.min.y + y1);
                        let size = egui::vec2(x2 - x1, y2 - y1);
                        let rect = egui::Rect::from_min_size(pos, size);
                        let response =
                            ui.interact(rect, ui.id().with("abs_pos"), egui::Sense::click());
                        (rect, response)
                    } else {
                        // 解析失败，使用默认布局
                        ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click())
                    }
                } else {
                    // 没有 pos 属性，使用默认布局
                    ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click())
                }
            } else {
                // 解析失败，使用默认布局
                ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click())
            }
        } else {
            // 默认布局流（非绝对定位）
            // NSIS 中 Button 的 padding 表示内容区域的内边距：
            // 总宽度 = left + button_width + right
            // 总高度 = top + button_height + bottom
            let total_width = width + pad_left + pad_right;
            let total_height = height + pad_top + pad_bottom;
            let (outer_rect, response) =
                ui.allocate_exact_size(egui::vec2(total_width, total_height), egui::Sense::click());

            let inner_rect = egui::Rect::from_min_size(
                egui::pos2(outer_rect.min.x + pad_left, outer_rect.min.y + pad_top),
                egui::vec2(width, height),
            );

            (inner_rect, response)
        };

        // 绘制按钮背景和文本
        // 检查是否有绝对定位（position 属性或 float="true" + pos 属性）
        let _is_absolute = element.attributes.get_custom("position").is_some()
            || element
                .attributes
                .get_custom("is_absolute")
                .map(|s| s == "true")
                .unwrap_or(false)
            || (element
                .attributes
                .get_custom("float")
                .map(|s| s == "true")
                .unwrap_or(false)
                && element.attributes.get_custom("pos").is_some());

        if ui.is_rect_visible(rect) {
            let inline_icon = Self::resolve_inline_button_icon(element, enabled, &response);
            let suppress_bg_image = matches!(
                inline_icon.as_ref().map(|icon| &icon.glyph),
                Some(InlineButtonGlyph::ChevronDown | InlineButtonGlyph::ChevronUp)
            );
            // 从 custom 属性读取图片配置
            let normalimage = element.attributes.get_custom("normalimage");
            let hotimage = element.attributes.get_custom("hotimage");
            let pushedimage = element.attributes.get_custom("pushedimage");
            let disabledimage = element.attributes.get_custom("disabledimage");

            // 调试日志已移除，避免每帧都输出（egui 的 update 函数会被频繁调用）
            // 如果需要调试，可以使用 tracing::debug! 并设置日志级别

            // 根据按钮状态选择背景图片
            let bg_image = if suppress_bg_image {
                None
            } else if !enabled {
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
                if let Some(texture) =
                    self.resource_cache
                        .get_background(ui.ctx(), &self.dpi_config, &image_path.path)
                {
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
                            egui::pos2(uv_max_x, uv_max_y),
                        )
                    } else {
                        // 使用整个纹理
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))
                    };

                    // 应用 fade 透明度
                    let image_color = if let Some(fade) = image_path.fade {
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, (fade * 255.0) as u8)
                    } else {
                        egui::Color32::WHITE
                    };

                    // 对于有 dest 裁剪的图片（如箭头图标），需要调整渲染位置
                    // 如果 dest 指定了裁剪区域，图片应该只显示裁剪的部分
                    if let Some((x1, y1, x2, y2)) = image_path.dest {
                        // 计算裁剪区域的尺寸
                        let crop_width = x2 - x1;
                        let crop_height = y2 - y1;
                        // 如果裁剪区域小于按钮尺寸，需要调整渲染矩形
                        let render_rect = if crop_width < width || crop_height < height {
                            // 裁剪区域较小，可能需要调整位置（如箭头图标在右侧）
                            // 对于自定义安装按钮，箭头图标应该在右侧
                            let text_padding_right = element
                                .attributes
                                .get_custom("textpadding")
                                .and_then(|s| {
                                    let parts: Vec<&str> = s.split(',').collect();
                                    if parts.len() >= 3 {
                                        parts[2].trim().parse::<f32>().ok()
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or(0.0);

                            if text_padding_right > 0.0 {
                                // 箭头图标在右侧
                                let icon_width = crop_width.min(width);
                                let icon_height = crop_height.min(height);
                                let icon_x = rect.max.x - icon_width;
                                let icon_y = rect.center().y - icon_height / 2.0;
                                egui::Rect::from_min_size(
                                    egui::pos2(icon_x, icon_y),
                                    egui::vec2(icon_width, icon_height),
                                )
                            } else {
                                // 默认居中
                                rect
                            }
                        } else {
                            rect
                        };

                        ui.painter()
                            .image(texture.id(), render_rect, uv_rect, image_color);
                    } else {
                        ui.painter().image(texture.id(), rect, uv_rect, image_color);
                    }
                } else {
                    // 图片加载失败，使用纯色背景
                    // 先获取圆角和边框配置，避免借用冲突
                    let corner_radius = self.get_corner_radius(element);
                    let border_stroke = self.get_border_stroke(element);
                    let bg_color = if enabled {
                        if response.hovered() {
                            Color32::from_rgb(70, 70, 70)
                        } else {
                            Color32::from_rgb(50, 50, 50)
                        }
                    } else {
                        Color32::from_rgb(30, 30, 30)
                    };
                    ui.painter().rect_filled(rect, corner_radius, bg_color);

                    // 绘制边框
                    if let Some(stroke) = border_stroke {
                        ui.painter().rect_stroke(
                            rect,
                            corner_radius,
                            stroke,
                            egui::epaint::StrokeKind::Outside,
                        );
                    }
                }
            } else {
                // 没有配置图片：不绘制背景（文本按钮）
            }

            // 绘制按钮文本
            // 根据按钮状态选择文本颜色（NSIS 格式：hottextcolor, pushedtextcolor, disabledtextcolor）
            let text_color = if !enabled {
                element
                    .attributes
                    .get_custom("disabledtextcolor")
                    .and_then(|c| self.parse_color(c))
                    .or_else(|| {
                        element
                            .attributes
                            .color
                            .as_ref()
                            .and_then(|c| self.parse_color(c))
                    })
                    .unwrap_or(Color32::GRAY)
            } else if response.is_pointer_button_down_on() {
                element
                    .attributes
                    .get_custom("pushedtextcolor")
                    .and_then(|c| self.parse_color(c))
                    .or_else(|| {
                        element
                            .attributes
                            .get_custom("hottextcolor")
                            .and_then(|c| self.parse_color(c))
                    })
                    .or_else(|| {
                        element
                            .attributes
                            .color
                            .as_ref()
                            .and_then(|c| self.parse_color(c))
                    })
                    .unwrap_or(Color32::WHITE)
            } else if response.hovered() {
                element
                    .attributes
                    .get_custom("hover-color")
                    .or_else(|| element.attributes.get_custom("hottextcolor"))
                    .and_then(|c| self.parse_color(c))
                    .or_else(|| {
                        element
                            .attributes
                            .color
                            .as_ref()
                            .and_then(|c| self.parse_color(c))
                    })
                    .unwrap_or(Color32::WHITE)
            } else {
                element
                    .attributes
                    .color
                    .as_ref()
                    .and_then(|c| self.parse_color(c))
                    .unwrap_or(Color32::WHITE)
            };

            let text_align_str = element
                .attributes
                .get_custom("textalign")
                .map(|s| s.as_str())
                .or_else(|| element.attributes.align.as_deref())
                .unwrap_or("center");
            let padding = Self::button_text_padding(element, text_align_str);

            let has_width_constraint = element
                .flex_style
                .as_ref()
                .map(|fs| {
                    !matches!(fs.max_width, crate::layout::dimension::Dimension::Auto)
                        || !matches!(fs.width, crate::layout::dimension::Dimension::Auto)
                })
                .unwrap_or_else(|| {
                    element.attributes.width.is_some() || element.attributes.max_width.is_some()
                });

            let (text_rect, icon_rect) =
                Self::compute_button_text_and_icon_rects(rect, padding, inline_icon.as_ref());

            if has_width_constraint {
                let galley = self.layout_button_text_galley(
                    ui,
                    text.clone(),
                    font_id.clone(),
                    text_color,
                    text_rect.width(),
                    true,
                );
                let galley_size = galley.size();
                let x = match text_align_str {
                    "left" => text_rect.min.x,
                    "right" => text_rect.max.x - galley_size.x,
                    _ => text_rect.min.x + (text_rect.width() - galley_size.x) / 2.0,
                };
                let y = text_rect.min.y + (text_rect.height() - galley_size.y) / 2.0;
                ui.painter().galley(egui::pos2(x, y), galley, text_color);
            } else {
                let galley = self.layout_button_text_galley(
                    ui,
                    text.clone(),
                    font_id.clone(),
                    text_color,
                    text_rect.width(),
                    false,
                );
                let galley_size = galley.size();
                let text_pos = match text_align_str {
                    "left" => {
                        egui::pos2(text_rect.min.x, text_rect.center().y - galley_size.y / 2.0)
                    }
                    "right" => egui::pos2(
                        text_rect.max.x - galley_size.x,
                        text_rect.center().y - galley_size.y / 2.0,
                    ),
                    _ => egui::pos2(
                        text_rect.center().x - galley_size.x / 2.0,
                        text_rect.center().y - galley_size.y / 2.0,
                    ),
                };
                ui.painter().galley(text_pos, galley, text_color);
            }

            if let (Some(icon), Some(icon_rect)) = (inline_icon.as_ref(), icon_rect) {
                match &icon.glyph {
                    InlineButtonGlyph::Image(image) => {
                        self.paint_image_in_rect(ui, image, icon_rect)
                    }
                    InlineButtonGlyph::ChevronDown => {
                        Self::paint_chevron_in_rect(ui, icon_rect, text_color, false)
                    }
                    InlineButtonGlyph::ChevronUp => {
                        Self::paint_chevron_in_rect(ui, icon_rect, text_color, true)
                    }
                }
            }
        }

        // 移除频繁的日志输出

        if response.clicked() && enabled {
            result.button_clicks.insert(id.clone(), true);
            // Capture action attribute if present
            if let Some(action) = element.attributes.get_custom("action") {
                result.button_actions.insert(id.clone(), action.clone());
            }
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

        // 获取字体配置（从 font_id 或 font_size）
        let font_id = self.get_font_id(element);

        // 获取文字颜色
        let text_color = element
            .attributes
            .color
            .as_ref()
            .and_then(|c| self.parse_color(c))
            .unwrap_or(Color32::WHITE);

        // 获取文本对齐方式（textalign 优先，否则使用 align）
        let text_align = element
            .attributes
            .get_custom("textalign")
            .map(|s| s.as_str())
            .or_else(|| element.attributes.align.as_deref())
            .unwrap_or("left");

        // 获取垂直对齐方式（valign）
        let valign = element
            .attributes
            .get_custom("valign")
            .map(|s| s.as_str())
            .unwrap_or("top");

        // 获取尺寸约束
        let width = element.attributes.width;
        let height = element.attributes.height;

        // 如果指定了宽度和高度，使用精确尺寸
        if let (Some(w), Some(h)) = (width, height) {
            let (rect, _response) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());

            if ui.is_rect_visible(rect) {
                // 根据 textalign 和 valign 确定文本位置
                let text_pos = match (text_align, valign) {
                    ("left", "top") => egui::pos2(rect.min.x, rect.min.y),
                    ("left", "center") | ("left", "vcenter") => {
                        egui::pos2(rect.min.x, rect.center().y)
                    }
                    ("left", "bottom") => egui::pos2(rect.min.x, rect.max.y),
                    ("center", "top") => egui::pos2(rect.center().x, rect.min.y),
                    ("center", "center") | ("center", "vcenter") => rect.center(),
                    ("center", "bottom") => egui::pos2(rect.center().x, rect.max.y),
                    ("right", "top") => egui::pos2(rect.max.x, rect.min.y),
                    ("right", "center") | ("right", "vcenter") => {
                        egui::pos2(rect.max.x, rect.center().y)
                    }
                    ("right", "bottom") => egui::pos2(rect.max.x, rect.max.y),
                    _ => rect.center(),
                };

                let align2 = match (text_align, valign) {
                    ("left", "top") => egui::Align2::LEFT_TOP,
                    ("left", "center") | ("left", "vcenter") => egui::Align2::LEFT_CENTER,
                    ("left", "bottom") => egui::Align2::LEFT_BOTTOM,
                    ("center", "top") => egui::Align2::CENTER_TOP,
                    ("center", "center") | ("center", "vcenter") => egui::Align2::CENTER_CENTER,
                    ("center", "bottom") => egui::Align2::CENTER_BOTTOM,
                    ("right", "top") => egui::Align2::RIGHT_TOP,
                    ("right", "center") | ("right", "vcenter") => egui::Align2::RIGHT_CENTER,
                    ("right", "bottom") => egui::Align2::RIGHT_BOTTOM,
                    _ => egui::Align2::CENTER_CENTER,
                };

                ui.painter()
                    .text(text_pos, align2, &text, font_id, text_color);
            }
        } else if let Some(w) = width {
            // 只指定了宽度
            ui.allocate_ui_with_layout(
                egui::vec2(w, 0.0),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    let rich_text = egui::RichText::new(&text).font(font_id).color(text_color);

                    let label = egui::Label::new(rich_text);
                    ui.add(label);
                },
            );
        } else {
            // 没有尺寸约束，使用默认渲染
            let rich_text = egui::RichText::new(&text).font(font_id).color(text_color);

            let label = egui::Label::new(rich_text);
            ui.add(label);
        }
    }

    /// 解析颜色字符串
    fn parse_color(&self, color_str: &str) -> Option<egui::Color32> {
        // 支持 #RRGGBB 和 #AARRGGBB 格式
        if color_str.starts_with('#') {
            if color_str.len() == 7 {
                // #RRGGBB
                let r = u8::from_str_radix(&color_str[1..3], 16).ok()?;
                let g = u8::from_str_radix(&color_str[3..5], 16).ok()?;
                let b = u8::from_str_radix(&color_str[5..7], 16).ok()?;
                return Some(egui::Color32::from_rgb(r, g, b));
            } else if color_str.len() == 9 {
                // #AARRGGBB
                let a = u8::from_str_radix(&color_str[1..3], 16).ok()?;
                let r = u8::from_str_radix(&color_str[3..5], 16).ok()?;
                let g = u8::from_str_radix(&color_str[5..7], 16).ok()?;
                let b = u8::from_str_radix(&color_str[7..9], 16).ok()?;
                return Some(egui::Color32::from_rgba_unmultiplied(r, g, b, a));
            }
        }
        // 支持 0xRRGGBB 和 0xAARRGGBB 格式
        if color_str.starts_with("0x") || color_str.starts_with("0X") {
            let hex_str = &color_str[2..];
            if hex_str.len() == 6 {
                // 0xRRGGBB
                let r = u8::from_str_radix(&hex_str[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex_str[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex_str[4..6], 16).ok()?;
                return Some(egui::Color32::from_rgb(r, g, b));
            } else if hex_str.len() == 8 {
                // 0xAARRGGBB
                let a = u8::from_str_radix(&hex_str[0..2], 16).ok()?;
                let r = u8::from_str_radix(&hex_str[2..4], 16).ok()?;
                let g = u8::from_str_radix(&hex_str[4..6], 16).ok()?;
                let b = u8::from_str_radix(&hex_str[6..8], 16).ok()?;
                return Some(egui::Color32::from_rgba_unmultiplied(r, g, b, a));
            }
        }
        None
    }

    /// 颜色解析 (静态方法版本, 用于新渲染路径)
    fn parse_color_static(color_str: &str) -> Option<egui::Color32> {
        if color_str.starts_with('#') {
            if color_str.len() == 7 {
                let r = u8::from_str_radix(&color_str[1..3], 16).ok()?;
                let g = u8::from_str_radix(&color_str[3..5], 16).ok()?;
                let b = u8::from_str_radix(&color_str[5..7], 16).ok()?;
                return Some(egui::Color32::from_rgb(r, g, b));
            } else if color_str.len() == 9 {
                let a = u8::from_str_radix(&color_str[1..3], 16).ok()?;
                let r = u8::from_str_radix(&color_str[3..5], 16).ok()?;
                let g = u8::from_str_radix(&color_str[5..7], 16).ok()?;
                let b = u8::from_str_radix(&color_str[7..9], 16).ok()?;
                return Some(egui::Color32::from_rgba_unmultiplied(r, g, b, a));
            }
        }
        if color_str.starts_with("0x") || color_str.starts_with("0X") {
            let hex_str = &color_str[2..];
            if hex_str.len() == 6 {
                let r = u8::from_str_radix(&hex_str[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex_str[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex_str[4..6], 16).ok()?;
                return Some(egui::Color32::from_rgb(r, g, b));
            } else if hex_str.len() == 8 {
                let a = u8::from_str_radix(&hex_str[0..2], 16).ok()?;
                let r = u8::from_str_radix(&hex_str[2..4], 16).ok()?;
                let g = u8::from_str_radix(&hex_str[4..6], 16).ok()?;
                let b = u8::from_str_radix(&hex_str[6..8], 16).ok()?;
                return Some(egui::Color32::from_rgba_unmultiplied(r, g, b, a));
            }
        }
        None
    }

    /// 获取圆角半径（从 borderround 属性）
    fn get_corner_radius(&self, element: &LayoutElement) -> egui::CornerRadius {
        if let Some(borderround_str) = element.attributes.get_custom("borderround") {
            let parts: Vec<&str> = borderround_str.split(',').map(|s| s.trim()).collect();
            if parts.len() == 2 {
                if let (Ok(x), Ok(y)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>()) {
                    // CornerRadius 字段是 u8 类型，需要转换
                    return egui::CornerRadius {
                        nw: x as u8,
                        ne: x as u8,
                        sw: y as u8,
                        se: y as u8,
                    };
                }
            }
        }
        egui::CornerRadius::ZERO
    }

    /// 获取边框描边（从 bordercolor 和 bordersize 属性）
    fn get_border_stroke(&self, element: &LayoutElement) -> Option<egui::Stroke> {
        let border_color = element
            .attributes
            .get_custom("bordercolor")
            .and_then(|c| self.parse_color(c))
            .unwrap_or(Color32::GRAY);

        let border_size = element
            .attributes
            .get_custom("bordersize")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);

        if border_size > 0.0 {
            Some(egui::Stroke::new(border_size, border_color))
        } else {
            None
        }
    }

    /// 获取字体 ID（从 font_id 或 font_size 属性）
    fn get_font_id(&self, element: &LayoutElement) -> egui::FontId {
        // 优先从 font_id 获取字体配置
        if let Some(font_id_str) = element.attributes.get_custom("font_id") {
            if font_id_str.parse::<u32>().is_ok() {
                // 从 custom 中获取字体配置
                if let (Some(font_name), Some(font_size_str), Some(font_bold_str)) = (
                    element.attributes.get_custom("font_name"),
                    element.attributes.get_custom("font_size"),
                    element.attributes.get_custom("font_bold"),
                ) {
                    if let Ok(font_size) = font_size_str.parse::<f32>() {
                        let is_bold = font_bold_str.parse::<bool>().unwrap_or(false);
                        return egui::FontId {
                            size: font_size,
                            family: if is_bold {
                                egui::FontFamily::Name(font_name.clone().into())
                            } else {
                                egui::FontFamily::Proportional
                            },
                        };
                    }
                }
            }
        }

        // 回退到 font_size
        let font_size = element
            .attributes
            .get_custom("font_size")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(12.0);

        egui::FontId::proportional(font_size)
    }

    /// 渲染复选框
    fn render_checkbox(&mut self, ui: &mut Ui, element: &LayoutElement, result: &mut RenderResult) {
        let _id = element
            .attributes
            .id
            .as_ref()
            .unwrap_or(&"".to_string())
            .clone();
        let text = self.get_display_text(&element.attributes);
        let _style_type = StyleType::from(&element.attributes);
        let _enabled = element.attributes.enabled.unwrap_or(true);

        // 解析文本，检查是否包含链接
        let segments = self.parse_text_with_links(&text);

        // 如果文本中包含链接，使用自定义渲染
        if segments
            .iter()
            .any(|s| matches!(s, TextSegment::Link { .. }))
        {
            self.render_checkbox_with_links(ui, element, &segments, result);
        } else {
            // 标准渲染（无链接）
            self.render_checkbox_simple(ui, element, &text, result);
        }
    }

    /// 渲染简单复选框（无内联链接）
    fn render_checkbox_simple(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        text: &str,
        result: &mut RenderResult,
    ) {
        let id = element
            .attributes
            .id
            .as_ref()
            .unwrap_or(&"".to_string())
            .clone();

        // 获取字体和颜色配置
        let font_size = element
            .attributes
            .get_custom("font_size")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(12.0);

        let text_color = element
            .attributes
            .color
            .as_ref()
            .and_then(|c| self.parse_color(c))
            .unwrap_or(Color32::WHITE);

        let enabled = element.attributes.enabled.unwrap_or(true);

        // 获取尺寸约束
        let width = element.attributes.width;
        let height = element.attributes.height;

        // 获取当前状态
        let current_checked = self
            .interaction_state
            .checkbox_states
            .get(&id)
            .copied()
            .unwrap_or(false);
        let mut checkbox_state = self
            .interaction_state
            .checkbox_states
            .entry(id.clone())
            .or_insert(current_checked)
            .clone();

        // 如果指定了宽度和高度，使用精确尺寸
        let response = if let (Some(w), Some(h)) = (width, height) {
            let (rect, response) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());

            // 添加调试日志
            static CHECKBOX_RECT_LOGGED: Lazy<Mutex<std::collections::HashSet<String>>> =
                Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
            let mut logged = CHECKBOX_RECT_LOGGED.lock().unwrap();
            if !logged.contains(&id) {
                let is_visible = ui.is_rect_visible(rect);
                eprintln!(
                    "[Checkbox渲染] Checkbox '{}' 矩形: {:?}, 可见={}",
                    id, rect, is_visible
                );
                logged.insert(id.clone());
            }
            drop(logged);

            if ui.is_rect_visible(rect) {
                // 从 custom 属性读取图片配置
                let normalimage = element.attributes.get_custom("normalimage");
                let normalhotimage = element.attributes.get_custom("normalhotimage");
                let selectedimage = element.attributes.get_custom("selectedimage");
                let selectedhotimage = element.attributes.get_custom("selectedhotimage");
                let disabledimage = element.attributes.get_custom("disabledimage");

                // 从 custom 属性读取 textpadding（格式：left,top,right,bottom）
                let text_padding_left = element
                    .attributes
                    .get_custom("textpadding")
                    .and_then(|s| s.split(',').next())
                    .and_then(|s| s.trim().parse::<f32>().ok())
                    .unwrap_or(20.0);

                // 使用图片渲染复选框
                // 根据 DPI 使用不同的复选框大小：1x = 16px, 2x = 32px
                let checkbox_size = 16.0;
                let checkbox_rect =
                    egui::Rect::from_min_size(rect.min, egui::Vec2::splat(checkbox_size));

                // 检查是否悬停
                let is_hovered = response.hovered();

                // 根据状态选择图片（优先级：悬停状态 > 选中状态 > 禁用状态 > 正常状态）
                let checkbox_image = if !enabled {
                    disabledimage.or(normalimage)
                } else if checkbox_state {
                    // 选中状态：优先使用 selectedhotimage（悬停时），否则使用 selectedimage
                    if is_hovered {
                        selectedhotimage
                            .or(selectedimage)
                            .or(normalhotimage)
                            .or(normalimage)
                    } else {
                        selectedimage.or(normalimage)
                    }
                } else {
                    // 未选中状态：优先使用 normalhotimage（悬停时），否则使用 normalimage
                    if is_hovered {
                        normalhotimage.or(normalimage)
                    } else {
                        normalimage
                    }
                };

                // 渲染复选框图片
                if let Some(img_path_str) = checkbox_image {
                    let image_path = Self::parse_image_path(img_path_str);
                    if let Some(texture) = self.resource_cache.get_background(
                        ui.ctx(),
                        &self.dpi_config,
                        &image_path.path,
                    ) {
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
                                egui::pos2(uv_max_x, uv_max_y),
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
                            egui::epaint::StrokeKind::Outside,
                        );

                        if checkbox_state {
                            ui.painter().rect_filled(
                                checkbox_rect.shrink(3.0),
                                egui::CornerRadius::same(1),
                                Color32::WHITE,
                            );
                        }
                    }
                } else {
                    // 没有配置图片，使用简单方框
                    ui.painter().rect_stroke(
                        checkbox_rect,
                        egui::CornerRadius::same(2),
                        egui::Stroke::new(1.0, Color32::GRAY),
                        egui::epaint::StrokeKind::Outside,
                    );

                    if checkbox_state {
                        ui.painter().rect_filled(
                            checkbox_rect.shrink(3.0),
                            egui::CornerRadius::same(1),
                            Color32::WHITE,
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
                    text_color,
                );
            }

            response
        } else {
            // 没有尺寸约束，使用默认 checkbox
            let checkbox = egui::Checkbox::new(
                &mut checkbox_state,
                egui::RichText::new(text).size(font_size).color(text_color),
            );

            ui.add(checkbox)
        };

        // 移除频繁的日志输出

        if response.clicked() {
            let new_checked = !current_checked;
            self.interaction_state
                .checkbox_states
                .insert(id.clone(), new_checked);
            result.checkbox_changes.insert(id.clone(), new_checked);
        }

        // 记录复选框响应
        if let Some(id) = element.attributes.id.as_ref() {
            result.checkbox_responses.insert(id.clone(), response);
        }
    }

    /// 渲染带内联链接的复选框
    fn render_checkbox_with_links(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        segments: &[TextSegment],
        result: &mut RenderResult,
    ) {
        let id = element
            .attributes
            .id
            .as_ref()
            .unwrap_or(&"".to_string())
            .clone();

        // 获取字体大小和颜色配置
        let font_size = element
            .attributes
            .get_custom("font_size")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(12.0);

        let text_color = element
            .attributes
            .color
            .as_ref()
            .and_then(|c| self.parse_color(c))
            .unwrap_or(Color32::WHITE);

        let link_color = Color32::from_rgb(0, 255, 232); // #00FFE8

        // 获取当前状态
        let mut current_checked = self
            .interaction_state
            .checkbox_states
            .get(&id)
            .copied()
            .unwrap_or(false);

        // 获取尺寸约束
        let width = element.attributes.width;
        let height = element.attributes.height;

        // 从 custom 属性读取 textpadding（格式：left,top,right,bottom）
        // 注意：textpadding 的第一个值是左侧间距，用于 checkbox 图片和文本之间的间距
        let text_padding_left = element
            .attributes
            .get_custom("textpadding")
            .and_then(|s| {
                let parts: Vec<&str> = s.split(',').collect();
                if parts.len() >= 1 {
                    parts[0].trim().parse::<f32>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(8.0); // 默认间距改为 8.0，而不是 20.0

        // 如果指定了宽度和高度，使用精确尺寸
        let _rect = if let (Some(_w), Some(_h)) = (width, height) {
            // 尺寸会在 horizontal 布局中自动处理
        } else {
            // 没有尺寸约束，使用默认布局
        };

        // 使用水平布局渲染 checkbox 和文本
        let checkbox_changed = ui
            .horizontal(|ui| {
                // 如果有图片配置，使用图片渲染 checkbox
                let normalimage = element.attributes.get_custom("normalimage");
                let normalhotimage = element.attributes.get_custom("normalhotimage");
                let selectedimage = element.attributes.get_custom("selectedimage");
                let selectedhotimage = element.attributes.get_custom("selectedhotimage");

                // 使用图片渲染复选框
                let checkbox_size = if self.dpi_config.use_2x { 32.0 } else { 16.0 };

                // 先分配区域以检测悬停状态
                let (checkbox_rect, checkbox_response) =
                    ui.allocate_exact_size(egui::Vec2::splat(checkbox_size), egui::Sense::click());
                let is_hovered = checkbox_response.hovered();

                // 根据状态选择图片（优先级：悬停状态 > 选中状态 > 正常状态）
                let checkbox_image = if current_checked {
                    // 选中状态：优先使用 selectedhotimage（悬停时），否则使用 selectedimage
                    if is_hovered {
                        selectedhotimage
                            .or(selectedimage)
                            .or(normalhotimage)
                            .or(normalimage)
                    } else {
                        selectedimage.or(normalimage)
                    }
                } else {
                    // 未选中状态：优先使用 normalhotimage（悬停时），否则使用 normalimage
                    if is_hovered {
                        normalhotimage.or(normalimage)
                    } else {
                        normalimage
                    }
                };

                // 渲染复选框图片
                if ui.is_rect_visible(checkbox_rect) {
                    if let Some(img_path_str) = checkbox_image {
                        let image_path = Self::parse_image_path(img_path_str);
                        if let Some(texture) = self.resource_cache.get_background(
                            ui.ctx(),
                            &self.dpi_config,
                            &image_path.path,
                        ) {
                            let uv_rect = if let Some((x1, y1, x2, y2)) = image_path.dest {
                                let tex_size = texture.size();
                                let uv_min_x = x1 / tex_size[0] as f32;
                                let uv_min_y = y1 / tex_size[1] as f32;
                                let uv_max_x = x2 / tex_size[0] as f32;
                                let uv_max_y = y2 / tex_size[1] as f32;
                                egui::Rect::from_min_max(
                                    egui::pos2(uv_min_x, uv_min_y),
                                    egui::pos2(uv_max_x, uv_max_y),
                                )
                            } else {
                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))
                            };

                            ui.painter().image(
                                texture.id(),
                                checkbox_rect,
                                uv_rect,
                                egui::Color32::WHITE,
                            );
                        }
                    }
                }

                // 处理点击事件
                if checkbox_response.clicked() {
                    current_checked = !current_checked;
                }

                // 添加文本左侧间距（checkbox 图片和文本之间的间距）
                // 注意：textpadding 的第一个值已经包含了 checkbox 图片的宽度，所以只需要添加额外的间距
                // 如果 textpadding_left 很大（如 40），说明已经包含了 checkbox 宽度，直接使用
                // 如果 textpadding_left 很小（如 8），说明只是额外间距
                let actual_padding = if text_padding_left > checkbox_size {
                    // textpadding 已经包含了 checkbox 宽度，减去 checkbox 宽度得到实际间距
                    text_padding_left - checkbox_size
                } else {
                    // textpadding 只是额外间距
                    text_padding_left
                };
                ui.add_space(actual_padding);

                // 记录复选框响应
                if let Some(elem_id) = element.attributes.id.as_ref() {
                    result
                        .checkbox_responses
                        .insert(elem_id.clone(), checkbox_response.clone());
                }

                // 渲染文本片段（包含可点击链接）
                for segment in segments {
                    match segment {
                        TextSegment::Text(text) => {
                            let label = egui::Label::new(
                                egui::RichText::new(text).size(font_size).color(text_color),
                            );
                            ui.add(label);
                        }
                        TextSegment::Link {
                            id: link_id,
                            text: link_text,
                        } => {
                            let link_rich_text = egui::RichText::new(link_text)
                                .size(font_size)
                                .color(link_color);

                            let button = egui::Button::new(link_rich_text)
                                .frame(false) // 无边框，像普通链接
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

                // 返回是否有变化（通过检查 current_checked 是否改变）
                let was_checked = self
                    .interaction_state
                    .checkbox_states
                    .get(&id)
                    .copied()
                    .unwrap_or(false);
                current_checked != was_checked
            })
            .inner;

        // 更新状态
        if checkbox_changed {
            self.interaction_state
                .checkbox_states
                .insert(id.clone(), current_checked);
            result.checkbox_changes.insert(id.clone(), current_checked);
        }
    }

    /// 渲染文本输入框（RichEdit）
    fn render_text_input(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        result: &mut RenderResult,
    ) {
        let id = element
            .attributes
            .id
            .as_ref()
            .unwrap_or(&"".to_string())
            .clone();
        let _style_type = StyleType::from(&element.attributes);
        let enabled = element.attributes.enabled.unwrap_or(true);

        // 获取 RichEdit 特有属性
        let readonly = element
            .attributes
            .get_custom("readonly")
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false);
        let multiline = element
            .attributes
            .get_custom("multiline")
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false);

        // 获取字体配置
        let font_id = self.get_font_id(element);

        // 先获取背景颜色、圆角和边框配置，避免借用冲突
        let bg_color = element
            .attributes
            .background
            .as_ref()
            .and_then(|c| self.parse_color(c))
            .or_else(|| Some(Color32::from_rgb(30, 30, 30)));
        let corner_radius = self.get_corner_radius(element);
        let border_stroke = self.get_border_stroke(element);

        // 获取当前内容
        let text_input_value = self
            .interaction_state
            .text_inputs
            .entry(id.clone())
            .or_insert_with(String::new);

        // 创建文本输入框
        let mut text_input = if multiline {
            egui::TextEdit::multiline(text_input_value)
        } else {
            egui::TextEdit::singleline(text_input_value)
        };

        // 设置字体
        text_input = text_input.font(font_id);

        // 设置只读
        if readonly || !enabled {
            text_input = text_input.interactive(false);
        }

        // 设置尺寸
        if let Some(width) = element.attributes.width {
            text_input = text_input.desired_width(width);
        }
        if let Some(height) = element.attributes.height {
            if multiline {
                text_input = text_input.desired_rows((height / 20.0) as usize);
            }
        }

        // 渲染背景（如果有）
        if let Some(bg_color) = bg_color {
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(
                    element.attributes.width.unwrap_or(ui.available_width()),
                    element
                        .attributes
                        .height
                        .unwrap_or(if multiline { 100.0 } else { 30.0 }),
                ),
                egui::Sense::click(),
            );
            ui.painter().rect_filled(rect, corner_radius, bg_color);

            // 绘制边框
            if let Some(stroke) = border_stroke {
                ui.painter().rect_stroke(
                    rect,
                    corner_radius,
                    stroke,
                    egui::epaint::StrokeKind::Outside,
                );
            }
        }

        let response = ui.add(text_input);

        if response.changed() {
            result
                .text_input_changes
                .insert(id.clone(), text_input_value.clone());
        }

        // 记录文本输入响应
        if let Some(id) = element.attributes.id.as_ref() {
            result.text_input_responses.insert(id.clone(), response);
        }
    }

    /// 渲染图片
    fn render_image(&mut self, ui: &mut Ui, element: &LayoutElement, _result: &mut RenderResult) {
        if let Some(icon) = &element.attributes.icon {
            if let Some(texture) =
                self.resource_cache
                    .get_background(ui.ctx(), &self.dpi_config, icon)
            {
                // 如果 XML 中指定了尺寸，使用指定的尺寸
                // 否则使用纹理的实际渲染尺寸（会自动处理 2x 资源）
                let size = if let (Some(w), Some(h)) =
                    (element.attributes.width, element.attributes.height)
                {
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
    fn render_progress_bar(
        &mut self,
        ui: &mut Ui,
        element: &LayoutElement,
        _result: &mut RenderResult,
    ) {
        // 获取 ProgressBar 补充属性
        let min = element
            .attributes
            .get_custom("min")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);
        let max = element
            .attributes
            .get_custom("max")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(100.0);
        let value = element
            .attributes
            .get_custom("value")
            .and_then(|s| s.parse::<f32>().ok())
            .or_else(|| element.attributes.progress.map(|p| p * (max - min) + min))
            .unwrap_or(0.0);

        // 计算进度值（0.0-1.0）
        let progress = if max > min {
            ((value - min) / (max - min)).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let _style_type = StyleType::from(&element.attributes);
        let _progress_style = self.style_engine.get_progress_style(&_style_type);

        let width = element.attributes.width.unwrap_or(ui.available_width());
        let height = element.attributes.height.unwrap_or(6.0);

        // 分配进度条区域
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            // 获取背景颜色
            let bg_color = element
                .attributes
                .background
                .as_ref()
                .and_then(|c| self.parse_color(c))
                .or_else(|| Some(Color32::from_rgb(40, 40, 40)));

            // 先绘制背景（未填充部分）
            if let Some(bg_color) = bg_color {
                ui.painter()
                    .rect_filled(rect, egui::CornerRadius::same(3), bg_color);
            } else {
                // 尝试加载背景图片
                let bg_image = "assets/progress_bg.png";
                if let Some(bg_texture) =
                    self.resource_cache
                        .get_background(ui.ctx(), &self.dpi_config, bg_image)
                {
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
                        Color32::from_rgb(40, 40, 40),
                    );
                }
            }

            // 绘制前景（填充部分）
            let progress_width = width * progress;
            if progress_width > 0.0 {
                let progress_rect =
                    Rect::from_min_size(rect.min, Vec2::new(progress_width, height));

                // 尝试使用 foreimage（前景图片）
                let foreimage_str = element
                    .attributes
                    .get_custom("foreimage")
                    .cloned()
                    .or_else(|| element.attributes.get_custom("bar-image").cloned());
                if let Some(foreimage_str) = foreimage_str {
                    let image_path = Self::parse_image_path(&foreimage_str);
                    if let Some(fg_texture) = self.resource_cache.get_background(
                        ui.ctx(),
                        &self.dpi_config,
                        &image_path.path,
                    ) {
                        // 如果指定了 dest 裁剪区域，计算 UV 坐标
                        let uv_rect = if let Some((x1, y1, x2, y2)) = image_path.dest {
                            let tex_size = fg_texture.size();
                            let uv_min_x = x1 / tex_size[0] as f32;
                            let uv_min_y = y1 / tex_size[1] as f32;
                            let uv_max_x = x2 / tex_size[0] as f32;
                            let uv_max_y = y2 / tex_size[1] as f32;
                            egui::Rect::from_min_max(
                                egui::pos2(uv_min_x, uv_min_y),
                                egui::pos2(uv_max_x, uv_max_y),
                            )
                        } else {
                            // 根据进度裁剪前景图片
                            egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(progress, 1.0))
                        };

                        ui.painter()
                            .image(fg_texture.id(), progress_rect, uv_rect, Color32::WHITE);
                    } else {
                        // 前景图片加载失败，使用纯色填充
                        ui.painter().rect_filled(
                            progress_rect,
                            egui::CornerRadius::same(3),
                            Color32::from_rgb(0, 196, 178),
                        );
                    }
                } else {
                    // 没有配置 foreimage，尝试使用默认前景图片
                    let fg_image = "assets/progress_fg.png";
                    if let Some(fg_texture) =
                        self.resource_cache
                            .get_background(ui.ctx(), &self.dpi_config, fg_image)
                    {
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
                            Color32::from_rgb(0, 196, 178),
                        );
                    }
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

            egui::Area::new(egui::Id::new(
                element
                    .attributes
                    .id
                    .as_ref()
                    .unwrap_or(&"overlay".to_string()),
            ))
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
                self.i18n_strings
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| text.clone())
            } else {
                text.clone()
            }
        } else {
            String::new()
        }
    }

    pub(crate) fn get_display_text_for_harness(&self, attrs: &ElementAttributes) -> String {
        self.get_display_text(attrs)
    }

    /// 获取按钮点击状态
    pub fn get_button_clicked(&self, id: &str) -> bool {
        self.interaction_state
            .button_clicks
            .get(id)
            .copied()
            .unwrap_or(false)
    }

    /// 获取复选框状态
    pub fn get_checkbox_checked(&self, id: &str) -> bool {
        self.interaction_state
            .checkbox_states
            .get(id)
            .copied()
            .unwrap_or(false)
    }

    /// 获取文本输入内容
    pub fn get_text_input_value(&self, id: &str) -> String {
        self.interaction_state
            .text_inputs
            .get(id)
            .cloned()
            .unwrap_or_default()
    }

    /// 获取当前所有文本输入/选择框缓存值。
    pub fn get_all_text_input_values(&self) -> HashMap<String, String> {
        self.interaction_state.text_inputs.clone()
    }

    /// 获取资源缓存的可变引用（用于 MessageBoxManager）
    pub fn get_resource_cache_mut(&mut self) -> &mut crate::ui::dpi_handler::ResourceCache {
        &mut self.resource_cache
    }

    /// 设置文本输入内容
    pub fn set_text_input_value(&mut self, id: &str, value: String) {
        self.interaction_state
            .text_inputs
            .insert(id.to_string(), value);
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
    /// 被点击按钮的 action 属性 (button_id → action string)
    pub button_actions: HashMap<String, String>,
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
    /// 下拉选择变更 (select_id → selected_value)
    pub select_changes: HashMap<String, String>,
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
    use crate::layout::dimension::Dimension;
    use crate::layout::style_props::{FlexDirection, FlexStyle};
    use crate::layout::taffy_bridge::TaffyBridge;

    fn make_renderer() -> LayoutRenderer {
        let config = InstallerConfig::default();
        let dpi_config = DpiConfig::new(&config);
        LayoutRenderer::new(dpi_config, HashMap::new())
    }

    fn make_renderer_with_config(width: u32, height: u32) -> LayoutRenderer {
        let mut config = InstallerConfig::default();
        config.ui.window_width = width;
        config.ui.window_height = height;
        let dpi_config = DpiConfig::new(&config);
        LayoutRenderer::new(dpi_config, HashMap::new())
    }

    #[test]
    fn test_layout_renderer_uses_config_size() {
        let renderer = make_renderer_with_config(800, 600);
        assert_eq!(renderer.dpi_config.window_width, 800.0);
        assert_eq!(renderer.dpi_config.window_height, 600.0);
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
    fn test_display_text_i18n() {
        let config = InstallerConfig::default();
        let dpi_config = DpiConfig::new(&config);
        let mut i18n = HashMap::new();
        i18n.insert("welcome.title".to_string(), "欢迎".to_string());

        let renderer = LayoutRenderer::new(dpi_config, i18n);

        assert_eq!(
            renderer.get_display_text(&ElementAttributes::new().with_text("@welcome.title")),
            "欢迎"
        );
        assert_eq!(
            renderer.get_display_text(&ElementAttributes::new().with_text("Normal text")),
            "Normal text"
        );
        assert_eq!(renderer.get_display_text(&ElementAttributes::new()), ""); // no text
    }

    #[test]
    fn test_parse_color_hex_formats() {
        let renderer = make_renderer();
        assert_eq!(
            renderer.parse_color("#FF0000"),
            Some(Color32::from_rgb(255, 0, 0))
        );
        assert_eq!(
            renderer.parse_color("#00FF00"),
            Some(Color32::from_rgb(0, 255, 0))
        );
        assert_eq!(
            renderer.parse_color("#FF112233"),
            Some(Color32::from_rgba_unmultiplied(0x11, 0x22, 0x33, 0xFF))
        );
        assert_eq!(
            renderer.parse_color("0xFF0000"),
            Some(Color32::from_rgb(255, 0, 0))
        );
        assert_eq!(renderer.parse_color("invalid"), None);
        assert_eq!(renderer.parse_color(""), None);
    }

    #[test]
    fn test_parse_image_path_simple() {
        let ip = LayoutRenderer::parse_image_path("assets/btn.png");
        assert_eq!(ip.path, "assets/btn.png");
        assert!(ip.dest.is_none());
        assert!(ip.corner.is_none());
        assert!(ip.fade.is_none());
    }

    #[test]
    fn test_parse_image_path_with_dest_and_fade() {
        let ip =
            LayoutRenderer::parse_image_path("file='assets/btn.png' dest='0,0,100,40' fade='128'");
        assert_eq!(ip.path, "assets/btn.png");
        assert_eq!(ip.dest, Some((0.0, 0.0, 100.0, 40.0)));
        assert!((ip.fade.unwrap() - 128.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_button_text_and_icon_rects_reserves_inline_icon_space() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(160.0, 40.0));
        let padding = (0.0, 4.0, 0.0, 4.0);
        let icon = InlineButtonIcon {
            glyph: InlineButtonGlyph::ChevronDown,
            size: 12.0,
            gap: 4.0,
        };

        let (text_rect, icon_rect) =
            LayoutRenderer::compute_button_text_and_icon_rects(rect, padding, Some(&icon));
        let icon_rect = icon_rect.expect("inline icon rect");

        assert_eq!(icon_rect.width(), 12.0);
        assert_eq!(icon_rect.height(), 12.0);
        assert_eq!(icon_rect.max.x, 160.0);
        assert_eq!(text_rect.max.x, 144.0);
        assert_eq!(text_rect.min.y, 4.0);
        assert_eq!(text_rect.max.y, 36.0);
    }

    #[test]
    fn test_compute_button_text_and_icon_rects_without_icon_uses_full_content_rect() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(160.0, 40.0));
        let padding = (8.0, 4.0, 8.0, 4.0);

        let (text_rect, icon_rect) =
            LayoutRenderer::compute_button_text_and_icon_rects(rect, padding, None);

        assert!(icon_rect.is_none());
        assert_eq!(text_rect.min.x, 8.0);
        assert_eq!(text_rect.max.x, 152.0);
        assert_eq!(text_rect.min.y, 4.0);
        assert_eq!(text_rect.max.y, 36.0);
    }

    #[test]
    fn test_taffy_layout_uses_config_dimensions() {
        // 验证: render_with_taffy 使用 dpi_config.window_width/height 而非硬编码
        let renderer = make_renderer_with_config(800, 600);

        let mut page = LayoutElement::new(ElementType::Page);
        page.flex_style = Some(FlexStyle {
            width: Dimension::Percent(100.0),
            height: Dimension::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        });
        let mut child = LayoutElement::with_attributes(
            ElementType::Label,
            ElementAttributes::new().with_id("lbl").with_text("test"),
        );
        child.flex_style = Some(FlexStyle {
            width: Dimension::Percent(100.0),
            height: Dimension::Px(30.0),
            ..Default::default()
        });
        page.children = vec![child];
        let tree = LayoutTree::new(page);

        // 直接调用 taffy_bridge 用 renderer 的配置尺寸
        let mut bridge = TaffyBridge::new();
        let layout = bridge.compute_layout(
            &tree,
            renderer.dpi_config.window_width,
            renderer.dpi_config.window_height,
        );
        let r = layout.get_rect("lbl").unwrap();
        assert_eq!(r.width, 800.0); // 100% of config width, not hardcoded 574
    }

    #[test]
    fn test_interaction_state_management() {
        let mut renderer = make_renderer();

        assert!(!renderer.get_checkbox_checked("cb1"));
        assert_eq!(renderer.get_text_input_value("input1"), "");

        renderer.set_text_input_value("input1", "hello".to_string());
        assert_eq!(renderer.get_text_input_value("input1"), "hello");

        renderer
            .interaction_state
            .checkbox_states
            .insert("cb1".to_string(), true);
        assert!(renderer.get_checkbox_checked("cb1"));

        renderer.clear_interaction_state();
        assert!(!renderer.get_checkbox_checked("cb1"));
        assert_eq!(renderer.get_text_input_value("input1"), "");
    }

    #[test]
    fn test_get_font_id_from_attributes() {
        let renderer = make_renderer();

        // 无 font_size 属性 → 默认 12.0
        let elem = LayoutElement::new(ElementType::Label);
        let font = renderer.get_font_id(&elem);
        assert_eq!(font.size, 12.0);

        // 有 font_size 属性
        let mut elem = LayoutElement::new(ElementType::Label);
        elem.attributes
            .custom
            .insert("font_size".to_string(), "18.0".to_string());
        let font = renderer.get_font_id(&elem);
        assert_eq!(font.size, 18.0);
    }

    #[test]
    fn test_get_corner_radius_from_borderround() {
        let renderer = make_renderer();

        // 无 borderround → ZERO
        let elem = LayoutElement::new(ElementType::Button);
        assert_eq!(renderer.get_corner_radius(&elem), CornerRadius::ZERO);

        // 有 borderround
        let mut elem = LayoutElement::new(ElementType::Button);
        elem.attributes
            .custom
            .insert("borderround".to_string(), "8,8".to_string());
        let cr = renderer.get_corner_radius(&elem);
        assert_eq!(cr.nw, 8);
        assert_eq!(cr.se, 8);
    }

    #[test]
    fn test_get_border_stroke_from_attributes() {
        let renderer = make_renderer();

        // 无 bordersize → None
        let elem = LayoutElement::new(ElementType::VBox);
        assert!(renderer.get_border_stroke(&elem).is_none());

        // bordersize=0 → None
        let mut elem = LayoutElement::new(ElementType::VBox);
        elem.attributes
            .custom
            .insert("bordersize".to_string(), "0".to_string());
        assert!(renderer.get_border_stroke(&elem).is_none());

        // bordersize=2, bordercolor=#FF0000 → Some
        let mut elem = LayoutElement::new(ElementType::VBox);
        elem.attributes
            .custom
            .insert("bordersize".to_string(), "2".to_string());
        elem.attributes
            .custom
            .insert("bordercolor".to_string(), "#FF0000".to_string());
        let stroke = renderer.get_border_stroke(&elem).unwrap();
        assert_eq!(stroke.width, 2.0);
        assert_eq!(stroke.color, Color32::from_rgb(255, 0, 0));
    }

    #[test]
    fn test_parse_text_with_links() {
        let renderer = make_renderer();

        let segments = renderer.parse_text_with_links("同意[用户协议](terms)和[隐私政策](privacy)");
        assert_eq!(segments.len(), 4);
        assert!(matches!(&segments[0], TextSegment::Text(t) if t == "同意"));
        assert!(
            matches!(&segments[1], TextSegment::Link { id, text } if id == "terms" && text == "用户协议")
        );
        assert!(matches!(&segments[2], TextSegment::Text(t) if t == "和"));
        assert!(
            matches!(&segments[3], TextSegment::Link { id, text } if id == "privacy" && text == "隐私政策")
        );
    }
}
