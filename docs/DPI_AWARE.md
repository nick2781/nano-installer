# DPI Aware 图片资源管理

## 概述

本项目采用标准的 1x/2x 双分辨率图片策略，根据系统 DPI 自动选择合适的图片资源。

## 图片资源结构

### 命名规范

```
assets/
├── logo.png          # 1x 标准分辨率 (96 DPI)
├── logo@2x.png       # 2x 高分辨率 (192+ DPI)
├── btn_primary.png   # 1x
├── btn_primary@2x.png # 2x
└── ...
```

### 分辨率说明

| DPI     | 缩放因子 | 使用图片    | 说明                   |
| ------- | -------- | ----------- | ---------------------- |
| 96 DPI  | 1.0x     | logo.png    | 标准分辨率             |
| 120 DPI | 1.25x    | logo.png    | 125% 缩放，仍使用 1x   |
| 144 DPI | 1.5x     | logo@2x.png | 150% 缩放，开始使用 2x |
| 192 DPI | 2.0x     | logo@2x.png | 200% 缩放，使用 2x     |

**切换阈值**：`scale_factor >= 1.5` 时使用 2x 图片

## 使用方法

### 1. 创建 AssetLoader

```rust
use crate::ui::AssetLoader;

// 从系统 DPI 创建（自动检测）
let loader = AssetLoader::from_system_dpi();

// 或手动指定缩放因子
let loader = AssetLoader::new(2.0);
```

### 2. 加载图片资源

```rust
use crate::ui::styles::assets;

// 获取 Logo 路径（自动选择 1x 或 2x）
let logo_path = loader.get_asset_path(assets::LOGO);

// 在标准 DPI 屏幕：返回 "assets/logo.png"
// 在高 DPI 屏幕：返回 "assets/logo@2x.png"
```

### 3. 在 GPUI 中使用

```rust
impl Render for InstallerApp {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let loader = AssetLoader::from_system_dpi();
        let logo_path = loader.get_asset_path(assets::LOGO);
        
        div()
            .child(
                img(logo_path)
                    .w(px(LOGO_WIDTH))
                    .h(px(LOGO_HEIGHT))
            )
    }
}
```

## AssetLoader API

### 方法

#### `new(scale_factor: f32) -> Self`
创建新的资源加载器，手动指定 DPI 缩放因子。

```rust
let loader = AssetLoader::new(1.5);
```

#### `from_system_dpi() -> Self`
从系统 DPI 自动创建加载器（推荐）。

```rust
let loader = AssetLoader::from_system_dpi();
```

#### `get_asset_path(&self, base_path: &str) -> PathBuf`
获取实际的资源路径，自动根据 DPI 选择 1x 或 2x。

```rust
let path = loader.get_asset_path("assets/logo.png");
// 返回: "assets/logo@2x.png" (如果是高 DPI)
```

#### `scale_factor(&self) -> f32`
获取当前的缩放因子。

```rust
let scale = loader.scale_factor(); // 例如: 2.0
```

#### `should_use_hidpi(&self) -> bool`
检查是否应该使用高清图片（scale_factor >= 1.5）。

```rust
if loader.should_use_hidpi() {
    println!("使用高清图片");
}
```

## 选择逻辑

```rust
pub fn get_asset_path(&self, base_path: &str) -> PathBuf {
    // 如果 scale_factor >= 1.5，尝试加载 2x 图片
    if self.scale_factor >= 1.5 {
        let path_2x = self.get_2x_path(base_path);
        if path_2x.exists() {  // 检查文件是否存在
            return path_2x;
        }
    }
    
    // 否则使用 1x 图片（默认）
    PathBuf::from(base_path)
}
```

**回退机制**：如果 2x 图片不存在，自动回退到 1x 图片。

## 为什么保留两套资源？

### 优势

1. **兼容低 DPI 屏幕**
   - 在标准 96 DPI 屏幕上使用 1x 图片
   - 避免加载过大的图片浪费内存
   - 加载速度更快

2. **优化高 DPI 显示**
   - 在高 DPI 屏幕（2K/4K）上使用 2x 图片
   - 显示清晰锐利，无模糊感
   - 提升用户体验

3. **文件大小对比**
   ```
   logo.png:        1 KB    (标准分辨率)
   logo@2x.png:     2 KB    (高分辨率，2倍大)
   
   bg_main.png:     196 KB  (标准)
   bg_main@2x.png:  715 KB  (高清，3.6倍大)
   ```

4. **内存优化**
   - 在 1080p 屏幕上，使用 1x 图片可节省 60% 内存
   - 在 4K 屏幕上，2x 图片显示清晰

### 如果只保留 2x 图片的问题

1. **浪费内存**：在低 DPI 屏幕上加载过大的图片
2. **加载变慢**：文件更大，I/O 时间增加
3. **缩放损失**：需要 GPU 缩小图片，可能产生锯齿

## 测试不同 DPI

### Windows 测试

```powershell
# 查看当前 DPI
$dpi = (Get-ItemProperty "HKCU:\Control Panel\Desktop\WindowMetrics").AppliedDPI
Write-Host "Current DPI: $dpi"

# 计算缩放因子
$scale = $dpi / 96
Write-Host "Scale Factor: $scale"
```

### 在代码中测试

```rust
#[test]
fn test_asset_loading() {
    // 测试标准 DPI (1.0x)
    let loader = AssetLoader::new(1.0);
    assert_eq!(
        loader.get_asset_path("assets/logo.png"),
        PathBuf::from("assets/logo.png")
    );
    
    // 测试高 DPI (2.0x)
    let loader = AssetLoader::new(2.0);
    // 会返回 @2x 版本（如果存在）
    let path = loader.get_asset_path("assets/logo.png");
    println!("High DPI path: {:?}", path);
}
```

## 创建新的图片资源

### 1. 准备图片

```bash
# 1x 版本 (标准分辨率)
logo.png          # 148x80 像素

# 2x 版本 (双倍分辨率)
logo@2x.png       # 296x160 像素 (尺寸 × 2)
```

### 2. 放置到 assets/ 目录

```bash
cp logo.png assets/
cp logo@2x.png assets/
```

### 3. 在代码中定义

```rust
// src/ui/styles/mod.rs
pub mod assets {
    pub const NEW_IMAGE: &str = "assets/new_image.png";
}
```

### 4. 使用

```rust
let loader = AssetLoader::from_system_dpi();
let path = loader.get_asset_path(assets::NEW_IMAGE);
```

## 资源优化建议

### 1. 使用 PNG 压缩

```bash
# 使用 pngquant 压缩 PNG（无损质量）
pngquant --quality=85-95 logo.png -o logo.png
pngquant --quality=85-95 logo@2x.png -o logo@2x.png
```

### 2. 检查文件大小

```bash
# 查看资源大小
du -h assets/*.png | sort -h

# 统计总大小
du -sh assets/
```

### 3. 只为大图片提供 2x 版本

对于非常小的图标（< 20x20px），可以只提供 2x 版本：
- 小图标缩放不明显
- 节省管理成本
- AssetLoader 会自动回退到可用的版本

## 当前资源清单

```
总计: 31 个文件
- PNG 图片: 29 个 (14 个 1x + 14 个 2x + 1 个 bg_color.png)
- ICO 图标: 2 个

1x 图片 (14):
  logo.png, bg_main.png, bg_installing.png, bg_color.png,
  btn_primary.png, btn_hover.png, btn_disabled.png, btn_close.png,
  btn_dialog.png, btn_dialog_primary.png,
  checkbox-0.png, checkbox-2.png,
  arrow-down.png, arrow-up.png, bar_installing.png

2x 图片 (14):
  logo@2x.png, bg_main@2x.png, bg_installing@2x.png,
  btn_primary@2x.png, btn_hover@2x.png, btn_disabled@2x.png, btn_close@2x.png,
  btn_dialog@2x.png, btn_dialog_primary@2x.png,
  checkbox-0@2x.png, checkbox-2@2x.png,
  arrow-down@2x.png, arrow-up@2x.png, bar_installing@2x.png

ICO 图标 (2):
  logo.ico, uninst.ico
```

## 总结

✅ **保留两套资源是正确的做法**：
- 兼容各种 DPI 设置
- 优化内存和性能
- 提供最佳显示质量
- 符合业界标准实践

✅ **AssetLoader 自动处理**：
- 自动检测系统 DPI
- 智能选择合适图片
- 回退机制保证兼容
- 简单易用的 API

🎯 **使用建议**：
- 始终提供 1x 和 2x 两个版本
- 2x 图片尺寸 = 1x 尺寸 × 2
- 使用 AssetLoader 加载所有图片资源
- 定期优化压缩图片文件

