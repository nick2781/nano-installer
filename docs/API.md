# API 文档

## 多语言 API

### 初始化

```rust
use nano_installer::i18n;

// 初始化语言系统
i18n::init("zh-CN")?;
```

### 翻译文本

```rust
// 简单翻译
let text = i18n::tr("welcome_title");

// 带参数的翻译
let text = i18n::tr_with_args("welcome_title", &[
    ("app_name", "MyApp"),
]);
```

### 切换语言

```rust
i18n::switch_locale("en-US")?;
```

## 安装引擎 API

### 创建安装引擎

```rust
use nano_installer::installer::{InstallEngine, InstallState};

let mut engine = InstallEngine::new("/path/to/install".to_string());
```

### 添加任务

```rust
use nano_installer::installer::tasks::*;

engine.add_task(Box::new(ExtractFilesTask {
    payload_data: vec![...],
}));

engine.add_task(Box::new(CreateShortcutsTask {
    app_name: "MyApp".to_string(),
    exe_path: "/path/to/app.exe".to_string(),
}));
```

### 执行安装

```rust
let manifest = engine.install().await?;
```

## 卸载引擎 API

### 创建卸载引擎

```rust
use nano_installer::uninstaller::UninstallEngine;
use std::path::Path;

let engine = UninstallEngine::from_install_path(
    Path::new("/path/to/app"),
    false, // keep_data
)?;
```

### 执行卸载

```rust
engine.uninstall().await?;
```

## 日志 API

### 初始化日志

```rust
use nano_installer::logger;

let log_path = logger::init(
    None,           // 日志目录（None 使用临时目录）
    "MyApp",        // 应用名称
    true,           // 是否为安装器（false 为卸载器）
)?;
```

### 记录步骤

```rust
use nano_installer::logger::{log_step, StepStatus};

log_step("Extract Files", StepStatus::Started);
// ... 执行操作 ...
log_step("Extract Files", StepStatus::Completed);
```

## 平台 API

### 检测系统语言

```rust
use nano_installer::common::platform;

let locale = platform::detect_system_locale();
```

### 检查权限

```rust
let is_admin = platform::is_elevated()?;
```

### 请求提升权限

```rust
if !is_admin {
    platform::request_elevation(&args)?;
}
```

## 资源 API

### 提取 Payload

```rust
use nano_installer::resources::PayloadExtractor;

let payload_data = PayloadExtractor::extract_embedded_payload()?;

PayloadExtractor::extract_7z_to_dir(&payload_data, install_path)?;
```

### 安装清单

```rust
use nano_installer::resources::InstallManifest;

let manifest = InstallManifest::new(
    "MyApp".to_string(),
    "1.0.0".to_string(),
    install_path.into(),
    "zh-CN".to_string(),
);

manifest.save(&manifest_path)?;
```

## 语言包构建 API

### 创建语言包

```rust
use nano_installer::i18n::LanguagePack;

let mut pack = LanguagePack::new("en-US".to_string());
pack.translations.insert("key".to_string(), "value".to_string());

// 序列化
let bytes = pack.to_bytes()?;

// 反序列化
let loaded = LanguagePack::from_bytes(&bytes)?;
```

### 从 JSON 创建

```rust
let json = r#"{"hello": "Hello", "world": "World"}"#;
let pack = LanguagePack::from_json("en-US".to_string(), json)?;
```

