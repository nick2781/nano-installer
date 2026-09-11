# 架构

## 总体模型

项目由构建期 CLI、共享核心库和三个预编译 x64 stub 组成：LZMA installer、zlib
installer 和 uninstaller。每次产品构建只选择一个 installer stub 和 uninstaller。

```text
product project                         nano-installer toolchain
installer_config.json ----+             nano-installer.exe
layouts / assets ----------+----------> resource validation and bundle
locales / scripts ---------+                       |
payload/app.7z ------------+                       v
                                      lzma-x64.exe + bundle
                                                   |
                                                   v
                                          <Product>_Setup.exe
```

`installer/lib` 同时被 CLI、安装器 stub 和卸载器 stub 使用。CLI 不应包含产品逻辑，
产品项目也不应修改 stub 来实现品牌差异。

## 构建流程

`nano-installer build --project <dir>` 执行：

1. 读取并校验 `installer_config.json`。
2. 校验 XML、资源引用和 `@2x` 配对。
3. 把 JSON locale 编译为 `.pak`。
4. 给卸载器 stub 注入配置、布局、资源、语言和脚本。
5. 签名生成后的卸载器（配置了 `--sign-script` 时）。
6. 压缩卸载器，把配置、UI 资源、语言、payload、卸载器和脚本组成分段 bundle。
7. 复制安装器 stub，替换图标和版本资源，追加 bundle，最后签名 setup。

中间卸载器位于 `<project>/.build/`，最终 setup 位于 `<project>/dist/`。

## 运行时流程

```text
stub start
  -> read embedded bundle
  -> load config and locale
  -> detect install/update/uninstall/silent mode
  -> render XML wizard or run silent path
  -> execute configured tasks and optional Rhai script
  -> write uninstall manifest
```

卸载器读取安装时写入的 manifest，以此删除文件、快捷方式和注册表项。产品脚本创建的
副作用必须通过脚本 API 记录，否则通用卸载器无法可靠清理。

## Stub 查找

CLI 按以下顺序查找配置或环境变量选择的 installer/uninstaller stub：

1. `NANO_INSTALLER_STUB_DIR`。
2. `nano-installer.exe` 所在目录。
3. 当前工作目录相对的 `target/<profile>` 和 `../../target/<profile>`。

因此 release bundle 可以把 CLI 和所选 stubs 平铺在同一目录，也可以由生产流水线显式设置
`NANO_INSTALLER_STUB_DIR`。

## 产品边界

- JSON 配置：跨产品都成立的安装能力。
- XML：页面结构、控件、默认状态和动作绑定。
- locale：所有用户可见文案。
- Rhai：渠道文件、私有注册表、URI scheme 等产品副作用。
- Rust runtime：稳定且跨产品复用的引擎能力。

具体判断规则见[配置与脚本边界](CONFIG_VS_SCRIPT.md)。
