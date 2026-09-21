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
| `is_cancelled()` | 用户要求停掉当前任务后为 `true` |
| `get_install_path()` | 当前安装目录 |
| `get_checkbox_value(id)` | 读取复选框；安装用布局里的控件 id（如 `chkShotcut`），卸载用 `keep_data` |
| `get_text_value(id)` | 读取文本框当前的值；页面没有这个控件时返回空串 |
| `get_choice_value(id)` | 读取下拉框或单选组当前的那一行；下拉框用控件 id，单选组用组的名字；页面没有时返回空串 |
| `is_component_selected(id)` | 这次运行是否装该组件 |
| `selected_components()` | 这次安装的组件名数组，按工程声明的顺序 |
| `get_mode()` | `"install"` 或 `"uninstall"` |
| `log_info(text)`、`log_warn(text)`、`log_error(text)` | 写入脚本日志 |

`cancel` 按钮，或对关闭提问回答"是"，都会要求正在跑的任务停下。运行时随后在当前这一步走完的检查点
放弃并撤回已经写下的内容，所以脚本想让自己的一步提前收尾时，就在循环里问 `is_cancelled()`，然后自己
返回。见[动作](XML_LAYOUT_GUIDE.md#动作)。

`get_text_value` 与 `get_choice_value` 读的是用户按下开始安装那一刻页面上的取值；静默运行没有页面，
两个原语读到的都是空串。

组件选择由同一条规则得出：基础载荷之外，工程用 `components.items` 切出来的组件里这次运行挑了哪些。
`is_component_selected(id)` 按组件名回答，`selected_components()` 按工程声明的顺序给出数组，规则见
[配置参考](CONFIG_REFERENCE.md#组件)。卸载没有页面也没有组件选择，两个原语一律给出 `false` 与空数组。
复选框本身也能用 `get_checkbox_value` 按组件名读到，但那是页面上的控件；要问装不装，用上面这两个。


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
| `get_tools_dir()` | 把 `resources.tools_dir` 打包进来的目录摊到磁盘上，返回它的路径 |
| `sleep_ms(milliseconds)` | 等待 |

`extract_payload*` 走的是内置流程那一套解压：payload 先落盘，交给对应的运行时展开，同时校验归档
里没有卸载程序、manifest 和符号链接。解的是基础载荷与这次选中的组件：归档按工程声明的顺序逐个展开，
两个归档带同一个相对路径会当场失败。

`get_tools_dir()` 把 `resources.tools_dir` 指到的目录摊进安装包自己的暂存目录，保留原来的相对路径，
再把目录路径返回给脚本，可以直接交给 `run_command`。一次运行只摊一次，再问一次拿到同一个路径。
没有打包工具的工程，以及向一个没带工具的安装包要工具的脚本，拿到的都是空字符串和一条日志告警，
安装照常继续，由脚本自己决定没有那个程序时怎么办。

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

## 文件关联

| 原语 | 说明 |
| --- | --- |
| `register_file_association(extension, prog_id, description, command, icon)` | 为当前用户登记一种文件类型 |
| `unregister_file_association(extension, prog_id)` | 交还这种文件类型 |

关联写在 `HKCU\Software\Classes` 下：扩展名指向程序 id，程序 id 带着资源管理器显示的类型名、图标，
以及打开文件的命令行。扩展名写不写前面那个点都行；`prog_id` 之后的三个文本参数都可以为空，为空就不写
这一处。`command` 是文件类型唯一不能缺的一项，它就是 Windows 执行的那条命令行，文件占位符要加引号，
例如 `"C:\Program Files\MyApp\App.exe" "%1"`。写下的每个键和值都会记入 manifest，卸载时据此
撤销。名字里带路径分隔符的调用会在写入之前就被拒绝，否则它会写到用户文件类型所在的 classes 树之外。

## 进程

| 原语 | 说明 |
| --- | --- |
| `is_process_running(name)` | 按可执行文件名判断 |
| `kill_process(name)` | 结束该进程 |
| `run_detached(command)` | 后台启动 |
| `run_command(command, args)` | 等待结束，返回退出码，失败返回 `-1` |

## 依赖与下载

| 原语 | 说明 |
| --- | --- |
| `dependency_installed(id)` | 按工程声明的规则问机器上有没有这个依赖 |
| `install_dependency(id)` | 缺就装上；返回装完之后规则是否认为它在，只在安装时可用 |
| `download_file(url, path)` | 把 URL 取到 `path` |
| `download_file_with_hash(url, path, sha256)` | 同上，取到的文件要哈希对得上才留下 |
| `sha256_of_file(path)` | 文件的 SHA-256，小写十六进制；读不到时返回空串 |

两个依赖原语读的是工程的 `dependencies.items`，见[配置参考](CONFIG_REFERENCE.md#依赖)：脚本决定
什么时候检查、要不要装，工程只说这个依赖是什么、怎么认。`install_dependency` 在卸载脚本里一律返回
`false`，卸载不移除依赖，也不替别的产品装东西。

下载走机器自己的 HTTP 栈，代理、证书和 TLS 设置就是这台机器上浏览器用的那一套；`https` 会用上
TLS 1.2。取到的文件先与 `sha256` 比一次，对不上就删掉并返回 `false`，不会留半份给下一步；两次
下载都没有进度输出，要在进度条上说明什么，由脚本自己在调用前后 `set_progress()`。

## 系统

| 原语 | 说明 |
| --- | --- |
| `get_env(name)` | 环境变量，缺失返回空串 |
| `set_env(name, value)` | 写入当前用户的环境变量，供之后启动的进程读取 |
| `remove_env(name)` | 删除这个环境变量 |
| `get_drives()` | 固定磁盘根目录数组，如 `["C:\\", "D:\\"]` |
| `get_drive_space(drive)` | `[可用, 总量]`，失败返回空数组 |
| `shell_notify()` | 通知 shell 刷新图标缓存 |
| `get_config_value(path)` | 读取 `installer_config.json`，如 `project.name` |
| `get_current_exe()`、`get_exe_dir()` | 当前安装包或卸载程序的路径与目录 |
| `is_elevated()` | 当前进程是否已提权 |
| `show_message(title, message)` | 提示框，画在安装窗口内 |
| `show_error(title, message)` | 错误框，画在安装窗口内 |
| `ask_yes_no(title, message)` | 询问框，返回 `bool` |
| `run_tracked_uninstall(start, end)` | 按 manifest 删除产品，仅卸载可用 |

`set_env` 写的是 `HKCU\Environment`，也就是 Windows 读新进程环境变量的地方：安装包把变量导出到
自己的环境里，进程一退出就没了，写在这里的变量则对之后启动的程序依然有效。这个键归 Windows 和机器上
的其他产品共用，所以 manifest 记的是值而不是键，卸载只把那个值撤回去，与 `...\CurrentVersion\Run`
的处理一致。

`show_message`、`show_error` 和 `ask_yes_no` 不弹系统对话框，而是在安装窗口内画出产品自己的卡片，
版面取自 `ui.dialog_layout`。脚本写的标题压在正文上方，询问框给两个答案，按钮文案来自语言文件的
`yes` 和 `no`，`ask_yes_no` 返回被点中的那个。脚本在工作线程上等这一下点击，所以卡片打开期间窗口
照常重绘，也照常接受点击。

没有东西可画时仍旧用系统框：静默安装、窗口还没打开就启动的脚本，以及压根没带对话框版面的工程。

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
