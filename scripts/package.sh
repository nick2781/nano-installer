#!/bin/bash
# 打包脚本 - 将 payload 嵌入到安装器中

set -e

if [ "$#" -lt 2 ]; then
    echo "Usage: $0 <app.7z> <output-installer.exe>"
    echo "Example: $0 myapp.7z MyAppSetup.exe"
    exit 1
fi

PAYLOAD=$1
OUTPUT=$2
INSTALLER="dist/installer.exe"

if [ ! -f "$INSTALLER" ]; then
    INSTALLER="dist/installer"
fi

if [ ! -f "$INSTALLER" ]; then
    echo "Error: Installer not found. Please run build.sh first."
    exit 1
fi

if [ ! -f "$PAYLOAD" ]; then
    echo "Error: Payload file not found: $PAYLOAD"
    exit 1
fi

echo "Packaging installer with payload..."
echo "  Installer: $INSTALLER"
echo "  Payload: $PAYLOAD"
echo "  Output: $OUTPUT"

# 复制安装器
cp "$INSTALLER" "$OUTPUT"

# 追加 payload
cat "$PAYLOAD" >> "$OUTPUT"

# 追加魔数和大小
PAYLOAD_SIZE=$(wc -c < "$PAYLOAD")
echo -n "PAYLOAD" >> "$OUTPUT"
printf "%016x" "$PAYLOAD_SIZE" | xxd -r -p >> "$OUTPUT"

echo "Packaging completed!"
echo "Final installer: $OUTPUT"

