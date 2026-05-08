@echo off
echo ========================================
echo Building routecode Portable Executable
echo ========================================

:: Check for virtual environment
if not exist venv (
    echo [!] Virtual environment not found. Creating one...
    python -m venv venv
)

:: Activate venv
echo [*] Activating virtual environment...
call venv\Scripts\activate

:: Install/Update dependencies
echo [*] Installing dependencies...
pip install -e .
pip install pyinstaller

:: Run PyInstaller
echo [*] Running PyInstaller...
pyinstaller --clean routecode.spec

echo.
echo ========================================
echo Build Complete!
echo The executable can be found in the "dist" folder.
echo ========================================
pause
