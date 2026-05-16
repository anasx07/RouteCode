@echo off
set "PATH=%PATH%;%USERPROFILE%\.cargo\bin"
echo Building RouteCode CLI in release mode...
cargo build -p routecode-cli --release
if %ERRORLEVEL% EQU 0 (
    echo.
    echo Build successful! 
    echo Binary located at: target\release\routecode-cli.exe
) else (
    echo.
    echo Build failed!
    exit /b %ERRORLEVEL%
)
