//! CSS 尺寸值解析
//!
//! 支持 px / % / auto 三种尺寸单位，以及 CSS 简写格式的 margin/padding 解析

use serde::{Deserialize, Serialize};

/// CSS 尺寸值
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Dimension {
    /// 像素值 (逻辑像素, 1x)
    Px(f32),
    /// 百分比 (0.0 - 100.0)
    Percent(f32),
    /// 自动尺寸
    Auto,
}

impl Default for Dimension {
    fn default() -> Self {
        Dimension::Auto
    }
}

impl Dimension {
    /// 解析尺寸字符串: "100", "100px", "50%", "auto"
    pub fn parse(s: &str) -> Option<Dimension> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("auto") {
            return Some(Dimension::Auto);
        }
        if let Some(pct) = s.strip_suffix('%') {
            return pct.trim().parse::<f32>().ok().map(Dimension::Percent);
        }
        if let Some(px) = s.strip_suffix("px") {
            return px.trim().parse::<f32>().ok().map(Dimension::Px);
        }
        // 纯数字默认为 px
        s.parse::<f32>().ok().map(Dimension::Px)
    }

    /// 转换为 taffy::Dimension
    pub fn to_taffy(self) -> taffy::Dimension {
        match self {
            Dimension::Px(v) => taffy::Dimension::Length(v),
            Dimension::Percent(v) => taffy::Dimension::Percent(v / 100.0),
            Dimension::Auto => taffy::Dimension::Auto,
        }
    }

    /// 转换为 taffy::LengthPercentage (不支持 Auto, fallback 为 0px)
    pub fn to_taffy_length_pct(self) -> taffy::LengthPercentage {
        match self {
            Dimension::Px(v) => taffy::LengthPercentage::Length(v),
            Dimension::Percent(v) => taffy::LengthPercentage::Percent(v / 100.0),
            Dimension::Auto => taffy::LengthPercentage::Length(0.0),
        }
    }

    /// 转换为 taffy::LengthPercentageAuto
    pub fn to_taffy_length_pct_auto(self) -> taffy::LengthPercentageAuto {
        match self {
            Dimension::Px(v) => taffy::LengthPercentageAuto::Length(v),
            Dimension::Percent(v) => taffy::LengthPercentageAuto::Percent(v / 100.0),
            Dimension::Auto => taffy::LengthPercentageAuto::Auto,
        }
    }
}

/// 四边值 (top, right, bottom, left)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Edges<T: Copy> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T: Copy + Default> Default for Edges<T> {
    fn default() -> Self {
        Self {
            top: T::default(),
            right: T::default(),
            bottom: T::default(),
            left: T::default(),
        }
    }
}

impl Edges<Dimension> {
    /// 所有边为 0px
    pub fn zero() -> Self {
        let z = Dimension::Px(0.0);
        Self { top: z, right: z, bottom: z, left: z }
    }

    /// 解析 CSS 简写格式的 margin/padding:
    /// - 1 值: "10" -> all sides 10px
    /// - 2 值: "10 20" -> top/bottom=10, left/right=20
    /// - 3 值: "10 20 30" -> top=10, left/right=20, bottom=30
    /// - 4 值: "10 20 30 40" -> top=10, right=20, bottom=30, left=40
    pub fn parse_shorthand(s: &str) -> Option<Edges<Dimension>> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        match parts.len() {
            1 => {
                let v = Dimension::parse(parts[0])?;
                Some(Edges { top: v, right: v, bottom: v, left: v })
            }
            2 => {
                let tb = Dimension::parse(parts[0])?;
                let lr = Dimension::parse(parts[1])?;
                Some(Edges { top: tb, right: lr, bottom: tb, left: lr })
            }
            3 => {
                let t = Dimension::parse(parts[0])?;
                let lr = Dimension::parse(parts[1])?;
                let b = Dimension::parse(parts[2])?;
                Some(Edges { top: t, right: lr, bottom: b, left: lr })
            }
            4 => {
                let t = Dimension::parse(parts[0])?;
                let r = Dimension::parse(parts[1])?;
                let b = Dimension::parse(parts[2])?;
                let l = Dimension::parse(parts[3])?;
                Some(Edges { top: t, right: r, bottom: b, left: l })
            }
            _ => None,
        }
    }

    /// 转换为 taffy::Rect<LengthPercentage>
    pub fn to_taffy_lp(&self) -> taffy::Rect<taffy::LengthPercentage> {
        taffy::Rect {
            top: self.top.to_taffy_length_pct(),
            right: self.right.to_taffy_length_pct(),
            bottom: self.bottom.to_taffy_length_pct(),
            left: self.left.to_taffy_length_pct(),
        }
    }

    /// 转换为 taffy::Rect<LengthPercentageAuto>
    pub fn to_taffy_lpa(&self) -> taffy::Rect<taffy::LengthPercentageAuto> {
        taffy::Rect {
            top: self.top.to_taffy_length_pct_auto(),
            right: self.right.to_taffy_length_pct_auto(),
            bottom: self.bottom.to_taffy_length_pct_auto(),
            left: self.left.to_taffy_length_pct_auto(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimension_parse() {
        assert_eq!(Dimension::parse("100"), Some(Dimension::Px(100.0)));
        assert_eq!(Dimension::parse("100px"), Some(Dimension::Px(100.0)));
        assert_eq!(Dimension::parse("50%"), Some(Dimension::Percent(50.0)));
        assert_eq!(Dimension::parse("auto"), Some(Dimension::Auto));
        assert_eq!(Dimension::parse("AUTO"), Some(Dimension::Auto));
        assert_eq!(Dimension::parse(""), None);
        assert_eq!(Dimension::parse("abc"), None);
    }

    #[test]
    fn test_dimension_to_taffy() {
        assert_eq!(Dimension::Px(100.0).to_taffy(), taffy::Dimension::Length(100.0));
        assert_eq!(Dimension::Percent(50.0).to_taffy(), taffy::Dimension::Percent(0.5));
        assert_eq!(Dimension::Auto.to_taffy(), taffy::Dimension::Auto);
    }

    #[test]
    fn test_edges_shorthand_1_value() {
        let edges = Edges::parse_shorthand("10").unwrap();
        assert_eq!(edges.top, Dimension::Px(10.0));
        assert_eq!(edges.right, Dimension::Px(10.0));
        assert_eq!(edges.bottom, Dimension::Px(10.0));
        assert_eq!(edges.left, Dimension::Px(10.0));
    }

    #[test]
    fn test_edges_shorthand_2_values() {
        let edges = Edges::parse_shorthand("10 20").unwrap();
        assert_eq!(edges.top, Dimension::Px(10.0));
        assert_eq!(edges.right, Dimension::Px(20.0));
        assert_eq!(edges.bottom, Dimension::Px(10.0));
        assert_eq!(edges.left, Dimension::Px(20.0));
    }

    #[test]
    fn test_edges_shorthand_4_values() {
        let edges = Edges::parse_shorthand("10 20 30 40").unwrap();
        assert_eq!(edges.top, Dimension::Px(10.0));
        assert_eq!(edges.right, Dimension::Px(20.0));
        assert_eq!(edges.bottom, Dimension::Px(30.0));
        assert_eq!(edges.left, Dimension::Px(40.0));
    }

    #[test]
    fn test_edges_shorthand_with_percent() {
        let edges = Edges::parse_shorthand("10 50%").unwrap();
        assert_eq!(edges.top, Dimension::Px(10.0));
        assert_eq!(edges.right, Dimension::Percent(50.0));
    }

    #[test]
    fn test_edges_shorthand_with_auto() {
        let edges = Edges::parse_shorthand("auto 10").unwrap();
        assert_eq!(edges.top, Dimension::Auto);
        assert_eq!(edges.right, Dimension::Px(10.0));
    }
}
