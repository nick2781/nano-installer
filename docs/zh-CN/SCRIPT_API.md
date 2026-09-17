# 自定义安装与卸载步骤

项目里放了 `scripts/install.rhai`，安装步骤就按这个脚本走；`scripts/uninstall.rhai` 也一样。两个
文件都没有时走内置流程。两个脚本共用同一组原语，只是入口不同。

脚本是项目数据而不是插件系统：原语固定，引擎带有操作数上限，失控循环不会挂死安装。

## 契约

- 安装脚本必须部署 `install.exe_name` 指定的可执行文件，否则 `finish_install` 报错并回滚。
- 卸载脚本应该调用 `run_tracked_uninstall(start, end)`，它按 manifest 删除快捷方式、注册表项与
  已装文件。脚本没调用时，驱动会在脚本跑完之后走一遍库里的回退流程，产品不会残留；调了它，删除
  时机与进度区间才交给脚本自己定。
- 脚本失败（`throw`，或原语返回失败且脚本 `return`）会回滚本次安装写下的文件、快捷方式和注册表
  值，并删除本次全新创建的安装目录。

## 编码

`scripts/install.rhai` 与 `scripts/uninstall.rhai` 必须是 UTF-8。

## 进度与状态

| 原语 | 说明 |
| --- | --- |
| `set_progress(percent)` | 设置进度，0-100，超出会截断 |
| `set_status(text)` | 显示字面文本，不做翻译 |
| `set_status_key(key)` | 显示 locale 键对应的文案，随语言切换 |
| `is_cancelled()` | 恒为 `false`；任务运行期间向导拒绝关闭 |
| `get_install_path()` | 当前安装目录 |
| `get_checkbox_value(id)` | 读取复选框；安装用布局里的控件 id（如 `chkShotcut`），卸载用 `keep_data` |
| `get_mode()` | `"install"` 或 `"uninstall"` |
| `log_info(text)`、`log_warn(text)`、`log_error(text)` | 写入脚本日志 |

脚本没有控制台输出。失败时最后 32 行日志会附在向导显示的错误后面。

## 文件与 payload

| 原语 | 说明 |
| --- | --- |
| `extract_payload()` | 解压 payload 到安装目录，进度 0-100 |
| `extract_payload_with_progress(start, end)` | 同上，指定进度区间 |
| `copy_uninstaller()` | 把 bundle 内的卸载程序写入安装目录 |
| `write_file(path, contents)` | 写入文本文件 |
| `create_dir(path)` | 递归创建目录 |
| `delete_dir(path)` | 递归删除目录；路径必须是驱动器下的绝对路径 |
| `delete_file(path)` | 删除文件 |
| `copy_file(source, target)` | 复制文件 |
| `file_exists(path)`、`is_dir(path)` | 存在性检查 |
| `get_file_size(path)` | 字节数，失败返回 `-1` |
| `read_text_file(path)` | 读取文本，失败返回空串 |
| `list_dir(path)` | 目录项名称数组 |
| `path_join(base, child)`、`path_parent(path)`、`path_filename(path)` | 路径拼接与拆分 |
| `get_temp_path()` | 临时目录 |
| `sleep_ms(milliseconds)` | 等待 |

`extract_payload*` 复用内置流程的解压：payload 先落盘，交给对应的运行时展开，同时校验归档里没有
卸载程序、manifest 和符号链接。

## 注册表

键格式为 `HKCU\...` 或 `HKLM\...`。脚本写入的值和键会记入 manifest，卸载时按记录删除。

| 原语 | 说明 |
| --- | --- |
| `reg_write_string(key, name, value)` | 写 `REG_SZ` |
| `reg_write_dword(key, name, value)` | 写 `REG_DWORD` |
| `reg_read(key, name)` | 读 `REG_SZ`，缺失返回空串 |
| `reg_key_exists(key)` | 键是否存在 |
| `reg_delete_value(key, name)` | 删除单个值 |
| `reg_delete_key(key)` | 删除键及其子键，并从待删记录中移除 |

写入 Windows 或其他产品共用的容器键（比如 `...\CurrentVersion\Run`）时，只记录这一个值；卸载也
只删这个值，不会删掉整个键。

## 快捷方式

| 原语 | 说明 |
| --- | --- |
| `create_desktop_shortcut(name, target)` | 桌面快捷方式 |
| `create_start_menu_shortcut(name, target, folder)` | 开始菜单快捷方式 |
| `create_uninstall_shortcut(name, target, folder)` | 卸载快捷方式，写法同上 |
| `delete_desktop_shortcut(name)` | 删除桌面快捷方式 |
| `delete_start_menu_folder(folder)` | 删除开始菜单目录 |

## 进程

| 原语 | 说明 |
| --- | --- |
| `is_process_running(name)` | 按可执行文件名判断 |
| `kill_process(name)` | 结束该进程 |
| `run_detached(command)` | 后台启动 |
| `run_command(command, args)` | 等待结束，返回退出码，失败返回 `-1` |

## 系统

| 原语 | 说明 |
| --- | --- |
| `get_env(name)` | 环境变量，缺失返回空串 |
| `get_drives()` | 固定磁盘根目录数组，如 `["C:\\", "D:\\"]` |
| `get_drive_space(drive)` | `[可用, 总量]`，失败返回空数组 |
| `shell_notify()` | 通知 shell 刷新图标缓存 |
| `get_config_value(path)` | 读取 `installer_config.json`，如 `project.name` |
| `get_current_exe()`、`get_exe_dir()` | 当前安装包或卸载程序的路径与目录 |
| `is_elevated()` | 当前进程是否已提权 |
| `show_message(title, message)` | 提示框 |
| `show_error(title, message)` | 错误框 |
| `ask_yes_no(title, message)` | 询问框，返回 `bool` |
| `run_tracked_uninstall(start, end)` | 按 manifest 删除产品，仅卸载可用 |

## 示例

最小形态的安装脚本：

```rhai
let install_path = get_install_path();
set_status_key("status.extracting");
if !extract_payload_with_progress(5.0, 50.0) {
    show_error("Installation Error", "Failed to extract files");
    return;
}
copy_uninstaller();
create_start_menu_shortcut(get_config_value("project.name"),
                           path_join(install_path, get_config_value("install.exe_name")),
                           get_config_value("project.name"));
set_status_key("status.install_complete");
set_progress(100.0);
```

完整示例见 `examples/TapTap/scripts/`。
