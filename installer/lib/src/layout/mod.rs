//! 布局系统模块
//! 
//! 负责解析XML布局文件并生成内部布局树

pub mod xml_parser;
pub mod layout_tree;
pub mod element;

pub use xml_parser::XmlParser;
pub use layout_tree::{LayoutTree, LayoutNode};
pub use element::{LayoutElement, ElementType, ElementAttributes};
