//! Taffy 布局引擎桥接
//!
//! 将 LayoutTree 转换为 Taffy 树，执行布局计算，输出每个节点的绝对坐标 Rect

use crate::layout::element::{LayoutElement, ElementType};
use crate::layout::layout_tree::LayoutTree;
use egui;
use std::collections::HashMap;

/// 计算后的布局结果 — 每个节点的绝对坐标矩形
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComputedRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl ComputedRect {
    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0, width: 0.0, height: 0.0 }
    }

    /// 转换为 egui::Rect (相对于给定的 offset)
    pub fn to_egui_rect(&self, offset: egui::Pos2) -> egui::Rect {
        egui::Rect::from_min_size(
            egui::Pos2::new(offset.x + self.x, offset.y + self.y),
            egui::Vec2::new(self.width, self.height),
        )
    }
}

/// 节点在 computed layout 中的信息
#[derive(Debug, Clone)]
pub struct ComputedNode {
    pub rect: ComputedRect,
    pub element_type: ElementType,
    pub id: Option<String>,
}

/// 计算后的完整布局
#[derive(Debug, Clone)]
pub struct ComputedLayout {
    /// 按节点 ID 索引的布局 (只包含有 id 的节点)
    pub by_id: HashMap<String, ComputedRect>,
    /// 所有节点的布局 (按遍历顺序)
    pub nodes: Vec<ComputedNode>,
    /// 容器尺寸
    pub container_width: f32,
    pub container_height: f32,
}

impl ComputedLayout {
    /// 获取指定 ID 的计算后矩形
    pub fn get_rect(&self, id: &str) -> Option<&ComputedRect> {
        self.by_id.get(id)
    }
}

/// Taffy 桥接器
pub struct TaffyBridge {
    tree: taffy::TaffyTree<()>,
    /// Taffy taffy::NodeId -> 对应的 LayoutElement 信息
    node_info: HashMap<taffy::NodeId, (ElementType, Option<String>)>,
}

impl TaffyBridge {
    pub fn new() -> Self {
        Self {
            tree: taffy::TaffyTree::new(),
            node_info: HashMap::new(),
        }
    }

    /// 从 LayoutTree 构建 Taffy 树并计算布局
    pub fn compute_layout(
        &mut self,
        layout_tree: &LayoutTree,
        container_width: f32,
        container_height: f32,
    ) -> ComputedLayout {
        self.tree = taffy::TaffyTree::new();
        self.node_info.clear();

        let root_id = self.build_node(&layout_tree.root);

        self.tree
            .compute_layout(
                root_id,
                taffy::Size {
                    width: taffy::AvailableSpace::Definite(container_width),
                    height: taffy::AvailableSpace::Definite(container_height),
                },
            )
            .ok();

        let mut layout = ComputedLayout {
            by_id: HashMap::new(),
            nodes: Vec::new(),
            container_width,
            container_height,
        };

        self.collect_layout(root_id, 0.0, 0.0, &mut layout);
        layout
    }

    /// 递归构建 Taffy 节点
    fn build_node(&mut self, element: &LayoutElement) -> taffy::NodeId {
        let mut style = self.element_to_taffy_style(element);

        // 隐藏元素不参与布局 (display: none)
        let visible = element.visual_style.as_ref().map_or(
            element.attributes.visible.unwrap_or(true),
            |vs| vs.visible,
        );
        if !visible {
            style.display = taffy::Display::None;
        }

        // 先递归构建子节点
        let child_ids: Vec<taffy::NodeId> = element
            .children
            .iter()
            .map(|child| self.build_node(child))
            .collect();

        let node_id = self
            .tree
            .new_with_children(style, &child_ids)
            .expect("failed to create taffy node");

        self.node_info.insert(
            node_id,
            (element.element_type.clone(), element.attributes.id.clone()),
        );

        node_id
    }

    /// 将 LayoutElement 的属性转换为 taffy::Style
    fn element_to_taffy_style(&self, element: &LayoutElement) -> taffy::Style {
        // 如果元素有新格式的 FlexStyle, 直接使用
        if let Some(flex_style) = &element.flex_style {
            return flex_style.to_taffy();
        }

        // 否则从旧格式的 ElementAttributes 转换 (向后兼容)
        self.legacy_attrs_to_taffy_style(element)
    }

    /// 旧格式 ElementAttributes -> taffy::Style (向后兼容)
    fn legacy_attrs_to_taffy_style(&self, element: &LayoutElement) -> taffy::Style {
        let attrs = &element.attributes;

        let flex_direction = match element.element_type {
            ElementType::VBox | ElementType::Page => taffy::FlexDirection::Column,
            _ => taffy::FlexDirection::Row,
        };

        let is_absolute = attrs
            .custom
            .get("is_absolute")
            .map(|s| s == "true")
            .unwrap_or(false);

        let position = if is_absolute {
            taffy::Position::Absolute
        } else {
            taffy::Position::Relative
        };

        let width = match attrs.width {
            Some(w) => taffy::Dimension::Length(w),
            None => taffy::Dimension::Auto,
        };

        let height = match attrs.height {
            Some(h) => taffy::Dimension::Length(h),
            None => taffy::Dimension::Auto,
        };

        let padding = if let Some((top, right, bottom, left)) = attrs.padding {
            taffy::Rect {
                top: taffy::LengthPercentage::Length(top),
                right: taffy::LengthPercentage::Length(right),
                bottom: taffy::LengthPercentage::Length(bottom),
                left: taffy::LengthPercentage::Length(left),
            }
        } else {
            taffy::Rect::zero()
        };

        let flex_grow = attrs.flex.unwrap_or(0.0);

        let mut inset: taffy::Rect<taffy::LengthPercentageAuto> = taffy::Rect {
            top: taffy::LengthPercentageAuto::Auto,
            right: taffy::LengthPercentageAuto::Auto,
            bottom: taffy::LengthPercentageAuto::Auto,
            left: taffy::LengthPercentageAuto::Auto,
        };
        if let Some((x, y)) = attrs.position {
            inset.left = taffy::LengthPercentageAuto::Length(x);
            inset.top = taffy::LengthPercentageAuto::Length(y);
        }

        let gap = attrs.spacing.unwrap_or(0.0);

        taffy::Style {
            display: taffy::Display::Flex,
            position,
            flex_direction,
            flex_grow,
            flex_shrink: 1.0,
            size: taffy::Size { width, height },
            min_size: taffy::Size {
                width: attrs.min_width.map_or(taffy::Dimension::Auto, taffy::Dimension::Length),
                height: taffy::Dimension::Auto,
            },
            max_size: taffy::Size {
                width: attrs.max_width.map_or(taffy::Dimension::Auto, taffy::Dimension::Length),
                height: taffy::Dimension::Auto,
            },
            padding,
            gap: taffy::Size {
                width: taffy::LengthPercentage::Length(gap),
                height: taffy::LengthPercentage::Length(gap),
            },
            inset,
            ..Default::default()
        }
    }

    /// 递归收集布局结果, 转换为绝对坐标
    fn collect_layout(
        &self,
        node_id: taffy::NodeId,
        parent_x: f32,
        parent_y: f32,
        layout: &mut ComputedLayout,
    ) {
        let taffy_layout = self.tree.layout(node_id).expect("layout not computed");
        let abs_x = parent_x + taffy_layout.location.x;
        let abs_y = parent_y + taffy_layout.location.y;

        let rect = ComputedRect {
            x: abs_x,
            y: abs_y,
            width: taffy_layout.size.width,
            height: taffy_layout.size.height,
        };

        if let Some((element_type, id)) = self.node_info.get(&node_id) {
            if let Some(id) = id {
                layout.by_id.insert(id.clone(), rect);
            }

            layout.nodes.push(ComputedNode {
                rect,
                element_type: element_type.clone(),
                id: id.clone(),
            });
        }

        let children = self.tree.children(node_id).unwrap_or_default();
        for child_id in children {
            self.collect_layout(child_id, abs_x, abs_y, layout);
        }
    }
}

impl Default for TaffyBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::element::ElementAttributes;
    use crate::layout::dimension::{Dimension, Edges};
    use crate::layout::style_props::{
        FlexStyle, FlexDirection, Position,
    };

    fn make_page_with_flex(children: Vec<LayoutElement>) -> LayoutTree {
        let mut page = LayoutElement::new(ElementType::Page);
        page.flex_style = Some(FlexStyle {
            width: Dimension::Percent(100.0),
            height: Dimension::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        });
        page.children = children;
        LayoutTree::new(page)
    }

    #[test]
    fn test_vbox_layout() {
        // VBox with 3 equal-height children
        let mut children = Vec::new();
        for i in 0..3 {
            let mut child = LayoutElement::with_attributes(
                ElementType::Label,
                ElementAttributes::new().with_id(format!("item{}", i)).with_text("test"),
            );
            child.flex_style = Some(FlexStyle {
                width: Dimension::Percent(100.0),
                height: Dimension::Px(40.0),
                ..Default::default()
            });
            children.push(child);
        }

        let tree = make_page_with_flex(children);
        let mut bridge = TaffyBridge::new();
        let layout = bridge.compute_layout(&tree, 574.0, 358.0);

        let r0 = layout.get_rect("item0").unwrap();
        let r1 = layout.get_rect("item1").unwrap();
        let r2 = layout.get_rect("item2").unwrap();

        assert_eq!(r0.y, 0.0);
        assert_eq!(r0.height, 40.0);
        assert_eq!(r1.y, 40.0);
        assert_eq!(r2.y, 80.0);
        assert_eq!(r0.width, 574.0); // 100% of 574
    }

    #[test]
    fn test_hbox_layout() {
        let mut hbox = LayoutElement::new(ElementType::HBox);
        hbox.flex_style = Some(FlexStyle {
            width: Dimension::Percent(100.0),
            height: Dimension::Px(40.0),
            flex_direction: FlexDirection::Row,
            ..Default::default()
        });

        let mut left = LayoutElement::with_attributes(
            ElementType::Label,
            ElementAttributes::new().with_id("left").with_text("L"),
        );
        left.flex_style = Some(FlexStyle {
            width: Dimension::Px(100.0),
            height: Dimension::Px(40.0),
            ..Default::default()
        });

        let mut right = LayoutElement::with_attributes(
            ElementType::Label,
            ElementAttributes::new().with_id("right").with_text("R"),
        );
        right.flex_style = Some(FlexStyle {
            width: Dimension::Px(200.0),
            height: Dimension::Px(40.0),
            ..Default::default()
        });

        hbox.children = vec![left, right];
        let tree = make_page_with_flex(vec![hbox]);

        let mut bridge = TaffyBridge::new();
        let layout = bridge.compute_layout(&tree, 574.0, 358.0);

        let l = layout.get_rect("left").unwrap();
        let r = layout.get_rect("right").unwrap();

        assert_eq!(l.x, 0.0);
        assert_eq!(l.width, 100.0);
        assert_eq!(r.x, 100.0);
        assert_eq!(r.width, 200.0);
    }

    #[test]
    fn test_flex_grow() {
        let mut hbox = LayoutElement::new(ElementType::HBox);
        hbox.flex_style = Some(FlexStyle {
            width: Dimension::Px(300.0),
            height: Dimension::Px(40.0),
            flex_direction: FlexDirection::Row,
            ..Default::default()
        });

        let mut fixed = LayoutElement::with_attributes(
            ElementType::Label,
            ElementAttributes::new().with_id("fixed").with_text("F"),
        );
        fixed.flex_style = Some(FlexStyle {
            width: Dimension::Px(100.0),
            height: Dimension::Px(40.0),
            ..Default::default()
        });

        let mut grow = LayoutElement::with_attributes(
            ElementType::Label,
            ElementAttributes::new().with_id("grow").with_text("G"),
        );
        grow.flex_style = Some(FlexStyle {
            height: Dimension::Px(40.0),
            flex_grow: 1.0,
            ..Default::default()
        });

        hbox.children = vec![fixed, grow];
        let tree = make_page_with_flex(vec![hbox]);

        let mut bridge = TaffyBridge::new();
        let layout = bridge.compute_layout(&tree, 574.0, 358.0);

        let g = layout.get_rect("grow").unwrap();
        assert_eq!(g.width, 200.0); // 300 - 100 = 200
    }

    #[test]
    fn test_percentage_sizing() {
        let mut child = LayoutElement::with_attributes(
            ElementType::Label,
            ElementAttributes::new().with_id("half").with_text("H"),
        );
        child.flex_style = Some(FlexStyle {
            width: Dimension::Percent(50.0),
            height: Dimension::Px(40.0),
            ..Default::default()
        });

        let tree = make_page_with_flex(vec![child]);
        let mut bridge = TaffyBridge::new();
        let layout = bridge.compute_layout(&tree, 400.0, 300.0);

        let r = layout.get_rect("half").unwrap();
        assert_eq!(r.width, 200.0); // 50% of 400
    }

    #[test]
    fn test_absolute_positioning() {
        let mut page = LayoutElement::new(ElementType::Page);
        page.flex_style = Some(FlexStyle {
            width: Dimension::Percent(100.0),
            height: Dimension::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        });

        let mut overlay = LayoutElement::with_attributes(
            ElementType::Overlay,
            ElementAttributes::new().with_id("overlay"),
        );
        overlay.flex_style = Some(FlexStyle {
            position: Position::Absolute,
            left: Dimension::Px(10.0),
            top: Dimension::Px(20.0),
            width: Dimension::Px(200.0),
            height: Dimension::Px(100.0),
            ..Default::default()
        });

        page.children = vec![overlay];
        let tree = LayoutTree::new(page);

        let mut bridge = TaffyBridge::new();
        let layout = bridge.compute_layout(&tree, 574.0, 358.0);

        let r = layout.get_rect("overlay").unwrap();
        assert_eq!(r.x, 10.0);
        assert_eq!(r.y, 20.0);
        assert_eq!(r.width, 200.0);
        assert_eq!(r.height, 100.0);
    }

    #[test]
    fn test_padding_and_gap() {
        let mut vbox = LayoutElement::new(ElementType::VBox);
        vbox.flex_style = Some(FlexStyle {
            width: Dimension::Px(200.0),
            height: Dimension::Auto,
            flex_direction: FlexDirection::Column,
            padding: Edges {
                top: Dimension::Px(10.0),
                right: Dimension::Px(20.0),
                bottom: Dimension::Px(10.0),
                left: Dimension::Px(20.0),
            },
            gap: 5.0,
            ..Default::default()
        });

        let mut c1 = LayoutElement::with_attributes(
            ElementType::Label,
            ElementAttributes::new().with_id("c1").with_text("1"),
        );
        c1.flex_style = Some(FlexStyle {
            width: Dimension::Percent(100.0),
            height: Dimension::Px(30.0),
            ..Default::default()
        });

        let mut c2 = LayoutElement::with_attributes(
            ElementType::Label,
            ElementAttributes::new().with_id("c2").with_text("2"),
        );
        c2.flex_style = Some(FlexStyle {
            width: Dimension::Percent(100.0),
            height: Dimension::Px(30.0),
            ..Default::default()
        });

        vbox.children = vec![c1, c2];
        let tree = make_page_with_flex(vec![vbox]);

        let mut bridge = TaffyBridge::new();
        let layout = bridge.compute_layout(&tree, 574.0, 358.0);

        let r1 = layout.get_rect("c1").unwrap();
        let r2 = layout.get_rect("c2").unwrap();

        // c1 内容宽度 = 200 - 20 - 20 = 160
        assert_eq!(r1.width, 160.0);
        // c1 的 y 位置 = padding-top = 10
        assert_eq!(r1.y, 10.0);
        // c2 的 y 位置 = 10 (padding-top) + 30 (c1 height) + 5 (gap) = 45
        assert_eq!(r2.y, 45.0);
    }

    #[test]
    fn test_legacy_attrs_compat() {
        // 测试旧格式属性也能正确计算布局
        let page = LayoutElement::new(ElementType::Page)
            .add_child(
                LayoutElement::with_attributes(
                    ElementType::VBox,
                    ElementAttributes::new()
                        .with_size(574.0, 358.0)
                        .with_spacing(10.0),
                )
                .add_child(
                    LayoutElement::with_attributes(
                        ElementType::Label,
                        ElementAttributes::new()
                            .with_id("lbl")
                            .with_text("Hello")
                            .with_size(200.0, 40.0),
                    ),
                ),
            );

        let tree = LayoutTree::new(page);
        let mut bridge = TaffyBridge::new();
        let layout = bridge.compute_layout(&tree, 574.0, 358.0);

        let r = layout.get_rect("lbl").unwrap();
        assert_eq!(r.width, 200.0);
        assert_eq!(r.height, 40.0);
    }

    #[test]
    fn test_computed_rect_to_egui_rect_with_offset() {
        let rect = ComputedRect { x: 10.0, y: 20.0, width: 100.0, height: 50.0 };
        let egui_rect = rect.to_egui_rect(egui::Pos2::new(5.0, 3.0));
        assert_eq!(egui_rect.min.x, 15.0);
        assert_eq!(egui_rect.min.y, 23.0);
        assert_eq!(egui_rect.width(), 100.0);
        assert_eq!(egui_rect.height(), 50.0);
    }

    #[test]
    fn test_computed_rect_to_egui_rect_zero_offset() {
        // 最常见的用法: offset 是窗口左上角 (0,0)
        let rect = ComputedRect { x: 50.0, y: 100.0, width: 200.0, height: 40.0 };
        let egui_rect = rect.to_egui_rect(egui::Pos2::ZERO);
        assert_eq!(egui_rect.min.x, 50.0);
        assert_eq!(egui_rect.min.y, 100.0);
        assert_eq!(egui_rect.max.x, 250.0);
        assert_eq!(egui_rect.max.y, 140.0);
    }

    #[test]
    fn test_taffy_output_used_directly_as_logical_coords() {
        // 核心场景: Taffy 在 1x 逻辑坐标系计算, 输出直接作为 egui 坐标 (不乘 scale)
        // 模拟: Page(574x358) 内一个按钮在底部
        let mut page = LayoutElement::new(ElementType::Page);
        page.flex_style = Some(FlexStyle {
            width: Dimension::Percent(100.0),
            height: Dimension::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        });

        // 顶部 spacer 占 300px
        let mut spacer = LayoutElement::new(ElementType::Spacer);
        spacer.flex_style = Some(FlexStyle {
            width: Dimension::Percent(100.0),
            height: Dimension::Px(300.0),
            ..Default::default()
        });

        // 底部按钮 40px
        let mut btn = LayoutElement::with_attributes(
            ElementType::Button,
            ElementAttributes::new().with_id("install_btn").with_text("Install"),
        );
        btn.flex_style = Some(FlexStyle {
            width: Dimension::Px(200.0),
            height: Dimension::Px(40.0),
            ..Default::default()
        });

        page.children = vec![spacer, btn];
        let tree = LayoutTree::new(page);

        // 用配置尺寸计算 (模拟 config.ui.window_width/height)
        let container_w = 574.0;
        let container_h = 358.0;
        let mut bridge = TaffyBridge::new();
        let layout = bridge.compute_layout(&tree, container_w, container_h);

        let btn_rect = layout.get_rect("install_btn").unwrap();
        // 按钮 y = spacer高度 300
        assert_eq!(btn_rect.y, 300.0);
        assert_eq!(btn_rect.width, 200.0);
        assert_eq!(btn_rect.height, 40.0);

        // 转为 egui rect, offset=0 → 坐标不变, 无 scale 乘法
        let egui_rect = btn_rect.to_egui_rect(egui::Pos2::ZERO);
        assert_eq!(egui_rect.min.y, 300.0); // 直接是逻辑像素, 不是 300*2=600
        assert_eq!(egui_rect.width(), 200.0);
    }

    #[test]
    fn test_custom_container_size() {
        // 验证: 不同的容器尺寸 (非 574x358) 正确工作
        let mut child = LayoutElement::with_attributes(
            ElementType::Label,
            ElementAttributes::new().with_id("lbl").with_text("Hi"),
        );
        child.flex_style = Some(FlexStyle {
            width: Dimension::Percent(100.0),
            height: Dimension::Px(30.0),
            ..Default::default()
        });

        let tree = make_page_with_flex(vec![child]);
        let mut bridge = TaffyBridge::new();

        // 用 800x600 而非 574x358
        let layout = bridge.compute_layout(&tree, 800.0, 600.0);
        let r = layout.get_rect("lbl").unwrap();
        assert_eq!(r.width, 800.0); // 100% of 800
        assert_eq!(r.height, 30.0);
    }
}
