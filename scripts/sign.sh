#!/bin/bash
# 代码签名脚本（Windows）

set -e

if [ "$#" -lt 1 ]; then
    echo "Usage: $0 <file-to-sign> [certificate-path] [password]"
    echo "Example: $0 MyAppSetup.exe cert.pfx mypassword"
    exit 1
fi

FILE=$1
CERT=${2:-""}
PASSWORD=${3:-""}

if [ ! -f "$FILE" ]; then
    echo "Error: File not found: $FILE"
    exit 1
fi

echo "Signing file: $FILE"

# 使用 signtool（需要 Windows SDK）
if command -v signtool &> /dev/null; then
    if [ -n "$CERT" ] && [ -n "$PASSWORD" ]; then
        signtool sign /f "$CERT" /p "$PASSWORD" /t http://timestamp.digicert.com /v "$FILE"
    else
        echo "Warning: Certificate or password not provided. Skipping signing."
        echo "Please provide certificate path and password as arguments."
    fi
else
    echo "Warning: signtool not found. Please install Windows SDK."
    echo "Download from: https://developer.microsoft.com/windows/downloads/windows-sdk/"
fi

echo "Signing completed!"

