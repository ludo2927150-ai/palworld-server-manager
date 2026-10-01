@echo off
rem Double-cliquez pour lancer Palworld Server Manager avec mises a jour en direct.
rem (recupere automatiquement les nouveaux commits pendant que l application tourne)
title Palworld Server Manager - dev live
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\dev-live.ps1"
echo.
echo L application est arretee. Appuyez sur une touche pour fermer.
pause >nul
