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

构建器与运行时是两个独立的程序。生成的安装包只包含运行时和你的项目数据，永远不含构建器。

构建器先看 payload 的头几个字节：`7z` 签名走 LZMA 运行时，`PK` 签名走 ZIP 运行时。接着它复制
对应的运行时，写入你配置的图标、版本信息和应用程序清单，再追加一份 bundle，里面装着 layouts、
assets、locales、scripts 与 payload，最后是自包含的卸载程序。payload 保持你给的压缩格式，
打包时不会再压一层。

## 生成的安装包内部

**启动。** bundle 的尾部有一份索引，运行时靠它找到每个条目，所以启动时不用把 payload 载入
内存；真正开始解压时，才按偏移读取 payload 的字节。

**界面。** 页面取自 `wizard.pages`（安装）和 `wizard.uninstall_pages`（卸载）。每个页面都用
Win32 绘制，PNG 交给 WIC 解码，透明通道用 GDI alpha blend 合成。目前支持：绝对定位的位图与文字
层，可嵌套的 `VBox`/`HBox`/`Content` 流式布局（内边距、外边距、百分比尺寸、`flex-wrap` 折行都
在内），进度条，复选框与可展开面板交互，可点击链接，就地文本编辑（选区、剪贴板与撤销栈），目录
选择框，运行时切换语言，没有边框、四角是圆的窗口，任务栏图标，双缓冲绘制，拖动、最小化和关闭。

**提问与提示。** 要用户回答的问题（如退出确认）和要用户看到的提示，都画在窗口内部，用的是
`ui.dialog_layout` 指定的那份布局，不交给系统弹窗。对话框就是一份普通布局，
`value-source="dialog:*"` 让它显示这次提问的正文和按钮文案；打开期间只有对话框上的控件响应点击，
`Enter`/`Escape` 分别对应确认和取消。脚本的 `show_message`、`show_error`、`ask_yes_no` 也画在
这张卡片上，被点中的那个按钮就是交还给脚本的答案。窗口按所在显示器的工作区居中，布局比桌面还大
时夹取到工作区内。

**任务。** 安装和卸载都在工作线程上跑，进度与页面切换靠一份共享状态发布出去；UI 线程收到刷新消息
后重绘。绘制始终留在持有窗口的那个线程上，所以解压期间窗口照样能响应。

**安装。** 文件先暂存，然后写入 manifest 和卸载注册表，再创建配置的快捷方式与自启动项。目标目录
里已有同一项目时按升级处理：被替换的文件记入回滚日志，新版 payload 里已经没有的文件会删掉，
失败就恢复到先前版本。

**卸载。** 先结束产品进程，再删除记录下来的快捷方式与文件；声明的用户数据，只有用户取消勾选保留
数据时才会删掉。Windows 不允许进程删除自己正在运行的镜像，所以卸载收尾时，卸载程序会把自己复制成
临时目录里的一个清理副本：副本等卸载程序退出后删掉它，再移除已经清空的安装目录，最后让一个很快
就结束的 `cmd` 脚本删掉自己。目录里还留着用户文件时就保留。

**自定义步骤。** 项目里放了 `scripts/install.rhai` 或 `scripts/uninstall.rhai`，安装和卸载步骤就
按脚本走。脚本引擎编译在 `nano-installer-core` 里，所以每个运行时都带着它；脚本原语复用与内置流程
相同的部署、回滚与 manifest 代码，另外设了操作数上限，脚本失控也不会把安装挂死。

## 工具链

`nano-installer-cli` 和 `nano-installer-gui` 都是薄前端，共用 core 里同一套检查与构建 API。参数解析
和 eframe 界面代码留在 core 之外，所以 GUI 的 egui、eframe、winit 依赖不会进入任何运行时或安装包。

`install.require_admin` 与 `ui.dpi_aware` 决定清单怎么写：安装包靠这份清单向 Windows 申请需要的
权限，也靠它告诉系统窗口自行缩放。Windows 在进程启动之前就会读取清单，所以这两项必须写成资源，
不能做成运行时选项。

缩放声明写在两个元素里：`dpiAware` 给 Windows 7/8/8.1 读，`dpiAwareness` 给 Windows 10 1607 及
以后读，后者取值 `PerMonitorV2, PerMonitor`，这样每个受支持的 Windows 版本都能用上它支持的最清晰
显示方式。运行时收到 `WM_DPICHANGED` 后按新显示器的比例重新排版整页，而不是让系统拉伸一张位图。

## 后续阶段

还差真实 Windows 7 SP1 机器上的生产验证，以及代码签名。
