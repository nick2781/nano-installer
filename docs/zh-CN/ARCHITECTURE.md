# 架构

## 从一个项目目录到单个可执行文件

```text
MyApp/
  installer_config.json + layouts/ + assets/ + locales/ + scripts/ + payload/
                                    |
                                    v
                     nano-installer-native-x64.exe
                                    |
                        按 payload 文件头选择运行时
                            /                       \
                lzma-stub-native.exe        zlib-stub-native.exe
                            \                       /
                            + bundle + 卸载程序
                                    |
                                    v
                             MyApp_Setup.exe
```

构建器与运行时是两个独立程序。生成的安装包只包含运行时和你的项目数据，永远不含构建器。

构建器读取 payload 的头几个字节：`7z` 签名选 LZMA 运行时，`PK` 签名选 ZIP 运行时。随后它复制
对应运行时，写入你配置的图标、版本信息与应用程序清单，追加一个包含 layouts、assets、locales、
scripts 与 payload 的 bundle，最后追加自包含卸载程序。payload 保持你提供的压缩格式，打包过程不会再加一层
压缩。

## 生成的安装包内部

**启动。** 运行时通过读取 bundle 尾部的索引定位每个条目，因此启动时不需要把 payload 载入内存；
开始解压时才按偏移读取 payload 字节。

**界面。** 页面来自 `wizard.pages`（安装）与 `wizard.uninstall_pages`（卸载）。每个页面用 Win32
绘制，PNG 由 WIC 解码，透明通道用 GDI alpha blend。当前支持：绝对定位的位图与文字层，可嵌套的
`VBox`/`HBox`/`Content` 流式布局（含内边距、外边距、百分比尺寸、`flex-wrap` 折行），进度条，
复选框与可展开面板交互，可点击链接，就地文本编辑（选区、剪贴板与撤销栈），目录选择框，运行时
切换语言，无边框圆角窗口，任务栏图标，双缓冲绘制，拖动、最小化和关闭。

**提问与提示。** 需要用户回答的问题（如退出确认）和必须让用户看到的提示，都由 `ui.dialog_layout`
指定的布局在窗口内部绘制，而不是交给系统弹窗。对话框是一份普通布局，`value-source="dialog:*"`
让它显示当次提问的正文与按钮文案；打开期间只有对话框上的控件响应点击，`Enter`/`Escape` 分别
对应确认与取消。窗口位置按所在显示器的工作区居中，并在布局比桌面更大时夹取到工作区内。

**任务。** 安装与卸载在工作线程上执行，通过共享状态发布进度与页面切换；UI 线程收到刷新消息后
重绘。绘制始终发生在持有窗口的线程上，解压期间窗口保持可响应。

**安装。** 文件先暂存，再写入 manifest 与卸载注册表，并创建配置的快捷方式与自启动项。目标目录
已有同一项目时按升级处理：被替换的文件记入回滚日志，新版 payload 不再包含的文件会被删除，
失败则恢复到先前版本。

**卸载。** 先终止产品进程，再删除记录的快捷方式与文件；声明的用户数据只在用户取消勾选保留数据
时才删除。Windows 不允许进程删除自己正在运行的镜像，因此卸载收尾会把自身复制成临时目录中的
清理副本，由副本等待卸载程序退出后删除它并移除已清空的安装目录；副本自身再交给一个短生命周期
的 `cmd` 脚本删除。目录里仍留有用户文件时保留目录。

**自定义步骤。** 项目带有 `scripts/install.rhai` 或 `scripts/uninstall.rhai` 时，安装与卸载步骤
由脚本决定。脚本引擎编译进 `nano-installer-core`，因此每个运行时都带有它；脚本原语复用与内置
流程相同的部署、回滚与 manifest 代码，并设有操作数上限，失控脚本不会挂死安装。

## 工具链

`nano-installer-cli` 与 `nano-installer-gui` 都是同一套 core 检查与构建 API 的薄前端。参数解析和
eframe 界面代码留在 core 之外，因此 GUI 的 egui、eframe、winit 依赖不会进入任何运行时或安装包。

清单由 `install.require_admin` 与 `ui.dpi_aware` 生成，让安装包向 Windows 申请所需权限，并告诉
系统窗口自行缩放。Windows 在进程启动前就会读取清单，这也是这两项必须写成资源而不是运行时选项的
原因。

缩放同时声明在两个元素里：`dpiAware` 供 Windows 7/8/8.1 读取，`dpiAwareness` 供 Windows 10
1607 及以后读取，后者取值 `PerMonitorV2, PerMonitor`，让每个受支持的版本都拿到它能提供的最清晰
行为。运行时收到 `WM_DPICHANGED` 后按新显示器的比例重新排版整页，而不是让系统拉伸一张位图。

## 后续阶段

真实 Windows 7 SP1 机器上的生产验证，以及代码签名。
