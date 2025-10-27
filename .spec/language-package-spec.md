# 语言包规范文档

## 支持语言
- English (`en‑US`)
- 简体中文 (`zh‑CN`)
- 繁体中文 (`zh‑TW`)
- 日本語 (`ja`)
- Tiếng Việt (`vi`)

## 翻译资源源格式（示例 JSON）
```json
{
  "welcome_title": "Welcome to the setup of MyApp",
  "next_button": "Next",
  "cancel_button": "Cancel"
}
```

> 约定：所有语言源文件必须包含同一组 key；值为 UTF‑8 文本。

## .pak 语言包格式
**命名：**
```
locales/
  en‑US.pak
  zh‑CN.pak
  zh‑TW.pak
  ja.pak
  vi.pak
```

**二进制结构建议：**
| 字段 | 描述 |
|---|---|
| 魔数 `LNGP` (4B) | 文件标识 |
| 版本 (2B) | 语言包版本 |
| 语言代码 | UTF‑8（如 `en-US`） |
| 校验 | CRC32 或 HMAC |
| 键值表 | 若干项：`len(key)+key+len(val)+val` |
| 可选 | 压缩/加密块，填充对齐 |

**安全：**
- 支持 zlib 压缩或 AES 加密（可选）。
- 启动时校验 CRC/HMAC；失败回退默认语言。
- 若允许第三方包，要求签名并标注“非官方”状态。

## 运行时 API（示意）
```rust
fn tr(key: &str) -> Cow<'static, str>;
fn load_locale(pak_bytes: &[u8]) -> Result<Bundle>;
fn current_locale() -> &'static str;
```

## 构建流程（示意 Rust 伪代码）
```rust
// 从 JSON 生成 .pak
fn build_language_pack(src_json: &Path, out_pak: &Path, locale: &str) -> Result<()> {
    let map = read_json(src_json)?;
    let mut buf = BinaryWriter::new();
    buf.write_magic(b"LNGP")?;
    buf.write_u16(1)?;       // 版本
    buf.write_string(locale)?;
    let checksum_pos = buf.reserve_u32()?; // 预留 CRC32
    for (k,v) in map { buf.write_string(&k)?; buf.write_string(&v)?; }
    let crc = crc32(buf.bytes_after(checksum_pos+4));
    buf.patch_u32(checksum_pos, crc)?;
    write_file(out_pak, compress(buf.into_bytes()))?;
    Ok(())
}
```

## 回退与缺失键策略
- 缺失键：返回默认语言值并记录告警。
- 运行时可开启严格模式：发现缺失即在日志中标红。
