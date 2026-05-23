@echo off
cd /d "%~dp0"
if exist "target\release\mimo-opt-desktop.exe" (
    start "" "target\release\mimo-opt-desktop.exe"
) else if exist "target\debug\mimo-opt-desktop.exe" (
    start "" "target\debug\mimo-opt-desktop.exe"
) else (
    cargo run --bin mimo-opt-desktop
)
