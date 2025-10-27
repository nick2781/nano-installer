# 多语言文本更新

## 更新时间
2024-10-27

## 来源
从 TapTap NSIS 安装器项目提取多语言文本

## 支持的语言

| 语言代码 | 语言名称         | 键数 | 状态 |
| -------- | ---------------- | ---- | ---- |
| en-US    | English          | 48   | ✅    |
| zh-CN    | 简体中文         | 48   | ✅    |
| zh-TW    | 繁体中文         | 48   | ✅    |
| ja       | 日本語           | 48   | ✅    |
| ko       | 한국어           | 48   | ✅    |
| ru       | Русский          | 48   | ✅    |
| es       | Español          | 48   | ✅    |
| pt       | Português        | 48   | ✅    |
| vi       | Tiếng Việt       | 48   | ✅    |
| th       | ไทย              | 48   | ✅    |
| id       | Bahasa Indonesia | 48   | ✅    |

## 文本分类

### 1. 应用信息 (2)
- `app.name` - 应用名称（运行时替换）
- `app.version` - 应用版本（运行时替换）

### 2. 欢迎页面 (2)
- `welcome.title` - 欢迎标题
- `welcome.description` - 欢迎描述

### 3. 许可协议页面 (5)
- `license.title` - 许可协议标题
- `license.accept` - "我已阅读并同意"
- `license.tos` - 服务条款
- `license.privacy` - 隐私政策
- `license.and` - "和"连接词

### 4. 安装路径页面 (5)
- `install_path.title` - 选择安装位置
- `install_path.description` - 描述文本
- `install_path.browse` - 浏览按钮
- `install_path.required_space` - 所需空间
- `install_path.available_space` - 可用空间

### 5. 安装选项 (3)
- `options.desktop_shortcut` - 创建桌面快捷方式
- `options.autostart` - 开机启动
- `options.custom` - 自定义选项

### 6. 安装进度页面 (3)
- `installing.title` - 正在安装
- `installing.description` - 进度描述
- `installing.status` - 安装状态

### 7. 完成页面 (3)
- `finish.title` - 安装完成
- `finish.description` - 完成描述
- `finish.launch` - 立即启动

### 8. 卸载 (5)
- `uninstall.title` - 卸载程序
- `uninstall.confirm` - 确认卸载
- `uninstall.keep_data` - 保留本地数据
- `uninstall.progress` - 正在卸载
- `uninstall.complete` - 卸载完成

### 9. 按钮 (9)
- `button.install` - 安装
- `button.install_now` - 立即安装
- `button.cancel` - 取消
- `button.next` - 下一步
- `button.back` - 上一步
- `button.finish` - 完成
- `button.close` - 关闭
- `button.ok` - 确定
- `button.uninstall` - 卸载
- `button.done` - 完成

### 10. 错误消息 (7)
- `error.admin_required` - 需要管理员权限
- `error.disk_space` - 磁盘空间不足
- `error.install_failed` - 安装失败
- `error.app_running` - 应用正在运行
- `error.path_illegal` - 路径非法
- `error.disk_full` - 磁盘已满
- `error.insufficient_space` - 空间不足

### 11. 消息 (3)
- `message.install_incomplete` - 安装未完成
- `message.updating` - 正在更新
- `message.update_complete` - 更新完成

## 示例翻译对比

### 欢迎标题 (welcome.title)
- 🇺🇸 English: "Welcome to TapTap Setup"
- 🇨🇳 简体中文: "TapTap 安装程序"
- 🇹🇼 繁体中文: "歡迎使用 TapTap 安裝程式"
- 🇯🇵 日本語: "TapTap セットアップへようこそ"
- 🇰🇷 한국어: "TapTap 설치에 오신 것을 환영합니다"
- 🇷🇺 Русский: "Добро пожаловать в установку TapTap"
- 🇪🇸 Español: "Bienvenido a la instalación de TapTap"
- 🇵🇹 Português: "Bem-vindo à instalação do TapTap"
- 🇻🇳 Tiếng Việt: "Chào mừng đến với cài đặt TapTap"
- 🇹🇭 ไทย: "ยินดีต้อนรับสู่การติดตั้ง TapTap"
- 🇮🇩 Bahasa Indonesia: "Selamat datang di Instalasi TapTap"

### 安装按钮 (button.install)
- 🇺🇸 English: "Install"
- 🇨🇳 简体中文: "安装"
- 🇹🇼 繁体中文: "安裝"
- 🇯🇵 日本語: "インストール"
- 🇰🇷 한국어: "설치"
- 🇷🇺 Русский: "Установить"
- 🇪🇸 Español: "Instalar"
- 🇵🇹 Português: "Instalar"
- 🇻🇳 Tiếng Việt: "Cài đặt"
- 🇹🇭 ไทย: "ติดตั้ง"
- 🇮🇩 Bahasa Indonesia: "Instal"

### 错误消息 (error.admin_required)
- 🇺🇸 English: "Administrator rights are required to install TapTap."
- 🇨🇳 简体中文: "需要管理员权限才能安装 TapTap。"
- 🇹🇼 繁体中文: "需要管理員權限才能安裝 TapTap。"
- 🇯🇵 日本語: "TapTap をインストールするには管理者権限が必要です。"
- 🇰🇷 한국어: "TapTap을 설치하려면 관리자 권한이 필요합니다."
- 🇷🇺 Русский: "Для установки TapTap требуются права администратора."
- 🇪🇸 Español: "Se requieren derechos de administrador para instalar TapTap."
- 🇵🇹 Português: "São necessários direitos de administrador para instalar o TapTap."
- 🇻🇳 Tiếng Việt: "Cần quyền quản trị viên để cài đặt TapTap."
- 🇹🇭 ไทย: "ต้องมีสิทธิ์ผู้ดูแลระบบเพื่อติดตั้ง TapTap"
- 🇮🇩 Bahasa Indonesia: "Hak administrator diperlukan untuk menginstal TapTap."

## 占位符

文本中使用了以下占位符，运行时会被替换：

- `{app_name}` - 应用名称
- `{app_version}` - 应用版本

示例：
```json
{
  "error.app_running": "{app_name} 正在运行，请退出后重试！"
}
```

运行时会替换为：
```
TapTap 正在运行，请退出后重试！
```

## 使用方法

### 在 Rust 代码中使用

```rust
use crate::i18n::Translator;

let translator = Translator::new("zh-CN")?;

// 获取简单文本
let title = translator.get("welcome.title");

// 使用占位符
let mut params = HashMap::new();
params.insert("app_name".to_string(), "TapTap".to_string());
let message = translator.get_with_params("error.app_running", &params);
```

### 添加新的翻译键

1. 在所有 5 个语言文件中添加相同的键
2. 确保所有语言都有对应的翻译
3. 使用点号 `.` 分隔命名空间，如 `page.section.key`

示例：
```json
{
  "new_page.title": "New Page Title",
  "new_page.description": "New page description..."
}
```

## 翻译质量

### ✅ 高质量翻译
- 所有文本都来自 TapTap 官方 NSIS 安装器
- 经过实际产品使用验证
- 符合目标语言习惯

### ✅ 完整性
- 5 种语言全部覆盖
- 每种语言都有 48 个键
- 没有缺失的翻译

### ✅ 一致性
- 所有语言文件结构相同
- 键名统一
- 格式一致

## 未来扩展

如需添加更多语言：

1. 创建新的 `locales/{language-code}.json` 文件
2. 复制 `en-US.json` 作为模板
3. 翻译所有文本
4. 更新 `src/i18n/mod.rs` 中的语言列表
5. 运行 `langpack-builder` 生成 `.pak` 文件

## 注意事项

⚠️ **重要**：
- 不要修改键名，只修改值
- 保持所有语言文件的键一致
- 使用 UTF-8 编码
- 确保 JSON 格式正确
- 占位符 `{name}` 要保持不变

## 验证

运行以下命令验证语言文件：

```bash
# 验证 JSON 格式
cd locales && for f in *.json; do python3 -m json.tool $f > /dev/null && echo "✅ $f" || echo "❌ $f"; done

# 检查键的一致性
python3 << 'EOF'
import json
from pathlib import Path

locales_dir = Path('locales')
all_keys = set()
lang_files = {}

for lang_file in locales_dir.glob('*.json'):
    with open(lang_file, 'r', encoding='utf-8') as f:
        data = json.load(f)
        lang_code = lang_file.stem
        lang_files[lang_code] = data
        all_keys.update(data.keys())

for lang_code, data in lang_files.items():
    missing = all_keys - set(data.keys())
    if missing:
        print(f"⚠️  {lang_code}: 缺失 {missing}")
    else:
        print(f"✅ {lang_code}: 完整 ({len(data)} 个键)")
EOF
```

## 相关文件

- `locales/*.json` - 语言源文件
- `src/i18n/mod.rs` - 多语言 API
- `src/i18n/langpack.rs` - `.pak` 格式定义
- `src/i18n/loader.rs` - 语言加载器
- `src/bin/langpack_builder.rs` - 语言包构建工具

## 参考

- NSIS 原始文件: `/Users/nick/work/cps/taptap-pc-electron/NSIS_SetupSkin/SetupScripts/`
  - `TapTap_CN/lang_strings.nsh` - 简体中文
  - `TapTap_Intl/lang_strings.nsh` - 国际版（10种语言）

---

**最后更新**: 2024-10-27  
**总键数**: 48  
**支持语言**: 11 种  
**总翻译数**: 528 (48 × 11)  
**状态**: ✅ 完成

