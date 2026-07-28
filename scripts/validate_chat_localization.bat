@echo off
setlocal
cd /d "%~dp0.."

echo Validando gate de localizacao do chat...
where py >nul 2>&1
if errorlevel 1 (
    echo Falta Python no PATH.
    exit /b 1
)

py scripts\chat_localization_gate.py
if errorlevel 1 (
    echo.
    echo FALHOU: gate de localizacao do chat nao passou.
    exit /b 1
)

echo.
echo OK: gate de localizacao do chat limpo.
exit /b 0
