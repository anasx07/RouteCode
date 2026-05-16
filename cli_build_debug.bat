@echo off
echo Building RouteCode CLI in Debug mode...
cargo build -p routecode-cli
if %ERRORLEVEL% EQU 0 (
    echo.
    echo Build successful! 
    echo Binary located at: target\debug\routecode-cli.exe
) else (
    echo.
    echo Build failed!
    exit /b %ERRORLEVEL%
)
