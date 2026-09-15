# Native architecture

```text
examples/TapTap
  installer_config.json + layouts + assets + locales + scripts + payload
                                |
                                v
                 nano-installer-native-x64.exe
                                |
                     selects by payload signature
                    /                           \
       lzma-stub-native.exe            zlib-stub-native.exe
                    \                           /
                     + bundle + native uninstaller
                                |
                                v
                         TapTap_Setup.exe
```

The builder and runtime are separate binaries and Cargo packages under `crates/`. Setup packages
contain a native runtime stub, never the builder. The LZMA package links only `sevenz-rust`, the
zlib package links only ZIP/Deflate, and the uninstaller links neither archive backend.

`nano-installer-cli` and `nano-installer-gui` are thin frontends over the same core inspection and
build APIs. Argument parsing and eframe UI code remain outside core.

The runtime parses the first configured XML page and renders through Win32, WIC, GDI alpha
blending, and Unicode `DrawTextW`. It currently supports absolute bitmap/text layers, the first
HBox/Content flex subset, checkbox and expandable-panel interaction, static link rendering,
runtime locale switching, locale key resolution, a borderless rounded window, taskbar icon,
double-buffered GDI painting, dragging, minimize, and close actions.

The next implementation stages are complete nested layout, link input, page transitions,
upgrades, task progress, complete rollback, script execution, shortcuts, and production validation
of the basic manifest-driven installation and removal workflow.

## Bundle caveat

The validation bundle currently stores resources and payload without another compression layer;
the payload remains in its original ZIP or 7z form. Runtime startup currently reads the complete
bundle once, then releases payload bytes before entering the UI loop. Before production use,
payload access should use offsets or memory mapping so startup does not duplicate the full payload.
