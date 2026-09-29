# 插件 ABI

插件是随安装包一起发出的一个 DLL，脚本按名字调用它。这是这个框架的扩展方式：不改框架本身，
形状和 NSIS 一样（`DllName::Function`），而规范写在 [`include/nano_plugin.h`](../../include/nano_plugin.h)
里：那份头文件就是契约。这一页是同一份契约的说明，把作者第一次读要用到的部分讲清楚。

插件需要的都在这里，别的都不用猜：插件是装在安装进程里的原生代码，想*读*什么、*查*什么，直接调
Win32 API 就行。它不可以做的，是背着安装器往机器上放文件或注册表值，见
[经宿主写入](#经宿主写入)。

## 三条规则

1. **文本进、文本出。** 脚本传进来一个字符串数组；插件想交回几个值，就调几次 `host->push`。字符串是
   UTF-16，宿主会把它收到的内容拷走，所以插件不用关心谁释放什么。
2. **写就走宿主。** `host->write_file` 与 `host->write_registry` 会像脚本原语那样把写下的东西记下来，
   卸载因此收得回去。绕过它们直接用 Win32 API 写，等于在机器上留下没人会删的文件和键。
3. **不猜。** 宿主在调用之前填好 `abi_version` 与 `struct_size`。插件不认识这个版本，就该返回非零，
   而不是去读可能并不存在的字段。

## 命名与打包

工程声明一个目录：

```toml
[resources]
plugins_dir = "plugins"
```

目录里每个 DLL 都进安装包，读回来时对一遍构建记下的摘要，第一次被用到时解到临时目录。插件按 DLL
自己的名字调用：

```rhai
let disk = plugin_call("tapcore::GetDiskId", ["C:\\"]);
let all = plugin_values("tapcore::ListDevices", []);
```

`tapcore::GetDiskId` 会加载 `plugins/tapcore.dll` 并调用导出 `GetDiskId`。不按名字调用的文件照样随包走，
插件的数据文件、它需要的第二个 DLL 就是这么带的。

构建会拒绝这个运行时永远加载不了的 DLL，并说清是哪个文件、为什么：32 位映像（这个运行时只有 64 位）、
ARM64 映像、不是 DLL 的可执行文件、或者根本不是 Windows 映像的文件。在构建时知道这件事才是检查的意义
所在：在别人的机器上得到一句「不是有效的 Win32 应用程序」，对作者没有任何帮助。

## 导出

```c
#include "nano_plugin.h"

NANO_PLUGIN_EXPORT int32_t NANO_PLUGIN_CALL Hello(nano_plugin_host *host, int32_t argc,
                                                  const wchar_t *const *argv) {
    if (host->abi_version != NANO_PLUGIN_ABI_VERSION) {
        return 1;
    }
    host->push(host, L"hello");
    return 0;   /* 0 是成功，别的值让脚本这次调用失败 */
}
```

一个插件可以导出任意多个函数，签名都是这一个。`argv` 按脚本写的顺序放着参数，每个是 UTF-16、以 NUL
结尾，宿主拥有、只在这次调用期间有效；`argc` 是参数的个数。

`NANO_PLUGIN_EXPORT` 是写 C 或 C++ 的人不能忘的那一半：少了它，DLL 照样编得出来、也加载得起来，可就是
没有这个名字的导出，而且什么都不说。Rust 是用 `#[no_mangle]` 说同一件事，所以 Rust 那份示例里看不到这个
宏。

## 服务

| 服务 | 作用 |
| --- | --- |
| `push(host, value)` | 给脚本要读的那组答案加一个值，按调用顺序排 |
| `log(host, level, message)` | 往这次运行的日志写一行，失败的安装留下的就是这份文件 |
| `window(host)` | 向导窗口；无窗口运行给 `0` |
| `cancelled(host)` | 用户是否已经要求这次运行停下 |
| `install_dir(host)` | 产品装到哪个目录 |
| `write_file(host, path, text)` | 往安装目录里写一个文本文件，并记下来 |
| `write_registry(host, key, name, kind, value)` | 写一个注册表值，并记下来 |
| `error_text(host)` | 上一次服务为什么失败，一句话 |

`log` 的级别是 `NANO_PLUGIN_LOG_ERROR`、`NANO_PLUGIN_LOG_WARN`、`NANO_PLUGIN_LOG_INFO`。
`error_text` 永远不是空指针，内容到下一次服务调用之前都有效。

## 经宿主写入

`write_file` 收绝对路径，或者相对于安装目录的路径。安装目录之外一律拒绝：卸载重放的那份 manifest 是
按这次运行记下来的东西生成的，落在外面就等于这个产品在机器上留下一个没人记账的文件。安装期间经宿主
写下的文件会进 manifest，因为安装自己的快照看得到它；注册表值则按 `reg_write_string` 的方式记录，
卸载收回的正是这一个值（键属于这个产品时收回整个键）。

`write_registry` 的 `kind` 是 `string`、`expand`、`dword`、`qword` 四种。别的类型，以及不属于这个产品的
键，都会被拒绝。

卸载期间没有 manifest 可记（正在重放的就是它），这和工程的卸载脚本一样。插件在卸载脚本里
一样可以调用：卸载程序带的是自己那份 bundle，工程的插件也随那份走。

## 失败

| 发生了什么 | 脚本看到什么 |
| --- | --- |
| 安装包没带这个名字的插件 | `the setup ships no plugin called `x`` |
| DLL 里没有这个导出 | `the plugin x.dll does not export `Function`` |
| 插件返回了非零 | `the plugin x.dll answered 3 from `Function``，旁边带上插件自己最后写的那行日志 |
| 某个宿主服务拒绝 | 脚本看到插件的失败码；插件自己用 `error_text` 读原因 |

调用做不成就让脚本失败，脚本想继续可以用 `try` 接住。这一点比其他原语严格是有意的：那些原语回答的是
这台机器的事，脚本自己决定怎么处理；而插件调用是脚本点名要的第三方代码，**没发生的调用不是答案**。

**插件不能让自己的异常或 panic 逃出导出函数。** Rust 在 panic 要穿过这种函数的边界时直接中止进程，
所以插件 panic 的那个安装包会以一次崩溃结束，日志里什么原因都没有。这是量出来的，不是假设。

## 插件做不到的事

- **加一个脚本原语。** 原语是固定的；插件是被脚本调用的东西，不会变成语言的一部分。
- **画向导自己的控件。** `window` 让插件把自己的对话框挂到向导下面，而那个窗口是插件自己的：版面引擎、
  高对比配色、读屏描述都到不了里面。
- **经宿主写安装目录之外**，理由同上。
- **在 Windows 7 上跑，而自己的导入在那边不成立。** 插件是产品带出去的代码，平台要求与安装包一致。

## 从哪里开始

`crates/nano-installer-plugin-sample` 是一份能用的 Rust 插件：它会回答一个值、把参数加起来、经宿主写一个
文件和一个注册表值、拒绝安装目录外的路径，也会按需要失败。`examples/plugin-c/sample.c` 是同一个插件的 C
版本，只用 MSVC 和那份头文件编出来，它证明这套 ABI 属于头文件，不属于某一种语言。安装包级用例会各装一次
带它们的安装包，把每个函数都驱动一遍，所以它们同时也是「ABI 与这一页说法一致」的持有者。
