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
       native-lzma-x64.exe             native-zlib-x64.exe
                    \                           /
                     + bundle + native uninstaller
                                |
                                v
                         TapTap_Setup.exe
```

The builder and runtime are separate binaries. Setup packages contain a native runtime stub, never
the builder. `native-uninst-x64.exe` is embedded in the bundle as a separate future execution
boundary.

The runtime parses the first configured XML page and renders through Win32, WIC, GDI alpha
blending, and Unicode `DrawTextW`. It currently supports absolute bitmap/text layers, locale key
resolution, a borderless rounded window, dragging, minimize, and close actions.

The next implementation stages are complete HBox/Content layout, checkbox/select rendering and
input, page transitions, payload extraction, install task execution, and uninstallation.

## Bundle caveat

The validation bundle currently stores resources and payload without another compression layer;
the payload remains in its original ZIP or 7z form. Runtime loading currently reads the complete
bundle into memory. Before production use, payload access should use offsets or memory mapping so
startup does not duplicate the full payload in RAM.
