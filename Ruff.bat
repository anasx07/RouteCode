@echo off
setlocal enabledelayedexpansion

:: --- Configuration ---
set VENV_PYTHON=.\venv\Scripts\python.exe
set SRC_DIR=src
set RUFF_OPTS=--exclude=__pycache__,.venv,venv,dist,build,.gemini

:: --- Header ---
echo.
echo  [96m==================================================== [0m
echo  [96m       routecode Linting ^& Formatting (Ruff)         [0m
echo  [96m==================================================== [0m
echo.

:: --- Check Venv ---
if not exist "%VENV_PYTHON%" (
    echo  [91m[!] Error: Virtual environment not found at .\venv [0m
    echo      Please run 'python -m venv venv' and install dependencies first.
    exit /b 1
)

:: --- Linting (Check & Fix) ---
echo  [93m[1/2] Running Lint Checks (Fixing safe issues)... [0m
"%VENV_PYTHON%" -m ruff check . --fix %RUFF_OPTS%
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo  [91m[!] Linting failed or found unfixable issues. [0m
) else (
    echo  [92m[+] Linting passed! [0m
)

echo.

:: --- Formatting ---
echo  [93m[2/2] Applying Code Formatting... [0m
"%VENV_PYTHON%" -m ruff format . %RUFF_OPTS%
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo  [91m[!] Formatting failed. [0m
) else (
    echo  [92m[+] Formatting applied successfully! [0m
)

echo.
echo  [96m==================================================== [0m
echo  [92m                  Done! [0m
echo  [96m==================================================== [0m
echo.
pause
