@echo off
setlocal
cd /d "%~dp0.."

echo Validando sintaxe Fluent e integridade do PT-BR...
cargo run -p veloren-client-i18n --features bin --bin i18n_check -- pt-BR
if errorlevel 1 (
    echo.
    echo FALHOU: nao gere nem publique uma release.
    exit /b 1
)

echo.
echo OK: PT-BR passou no i18n_check oficial.
exit /b 0
