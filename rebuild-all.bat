@echo off
echo ========================================
echo Nano-Installer Complete Rebuild Script
echo ========================================
echo.

echo [Step 1/5] Building nano-installer CLI (debug)...
cargo build --bin nano-installer
if %ERRORLEVEL% NEQ 0 (
    echo ERROR: Failed to build nano-installer CLI
    exit /b 1
)
echo Done!
echo.

echo [Step 2/5] Building lzma stub (debug)...
cargo build --bin lzma-x64-unicode
if %ERRORLEVEL% NEQ 0 (
    echo ERROR: Failed to build lzma stub (debug)
    exit /b 1
)
echo Done!
echo.

echo [Step 3/5] Building lzma stub (release)...
cargo build --release --bin lzma-x64-unicode
if %ERRORLEVEL% NEQ 0 (
    echo ERROR: Failed to build lzma stub (release)
    exit /b 1
)
echo Done!
echo.

echo [Step 4/5] Building uninst stub (release)...
cargo build --release --bin uninst
if %ERRORLEVEL% NEQ 0 (
    echo ERROR: Failed to build uninst stub
    exit /b 1
)
echo Done!
echo.

echo [Step 5/5] Building TapTap example installer...
cd examples\TapTap
..\..\target\debug\nano-installer.exe build
if %ERRORLEVEL% NEQ 0 (
    echo ERROR: Failed to build TapTap installer
    cd ..\..
    exit /b 1
)
cd ..\..
echo Done!
echo.

echo ========================================
echo Build completed successfully!
echo ========================================
echo.
echo Output: examples\TapTap\dist\TapTap_Setup.exe
echo.
