@echo off
set "MAESTRO_CLANG_TOOL=clang.exe"
call "%~dp0resolve-clang.cmd" %*
exit /b %ERRORLEVEL%
