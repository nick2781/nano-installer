//! 布局系统模块
//!
//! 负责解析XML布局文件并生成内部布局树

pub mod dimension;
pub mod element;
pub mod layout_tree;
pub mod style_props;
pub mod taffy_bridge;
pub mod xml_parser;

pub use dimension::Dimension;
pub use element::{ElementAttributes, ElementType, LayoutElement};
pub use layout_tree::{LayoutNode, LayoutTree};
pub use style_props::{FlexStyle, VisualStyle, WidgetProps};
pub use taffy_bridge::{ComputedLayout, ComputedRect, TaffyBridge};
pub use xml_parser::XmlParser;
