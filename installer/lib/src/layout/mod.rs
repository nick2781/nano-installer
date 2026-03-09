//! 布局系统模块
//!
//! 负责解析XML布局文件并生成内部布局树

pub mod xml_parser;
pub mod layout_tree;
pub mod element;
pub mod dimension;
pub mod style_props;
pub mod taffy_bridge;

pub use xml_parser::XmlParser;
pub use layout_tree::{LayoutTree, LayoutNode};
pub use element::{LayoutElement, ElementType, ElementAttributes};
pub use dimension::Dimension;
pub use style_props::{FlexStyle, VisualStyle, WidgetProps};
pub use taffy_bridge::{TaffyBridge, ComputedLayout, ComputedRect};
