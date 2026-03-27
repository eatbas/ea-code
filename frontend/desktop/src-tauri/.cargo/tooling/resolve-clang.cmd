@echo off
setlocal EnableExtensions

set "TOOL=%EA_CODE_CLANG_TOOL%"
if "%TOOL%"=="" (
  echo Missing clang tool name. 1>&2
  exit /b 1
)

set "COMPILER="

if defined EA_CODE_LLVM_BIN call :probe "%EA_CODE_LLVM_BIN%"
if defined LLVM_HOME call :probe "%LLVM_HOME%\bin"
if defined VCToolsInstallDir call :probe "%VCToolsInstallDir%Llvm\bin"
if defined VCINSTALLDIR call :probe "%VCINSTALLDIR%Tools\Llvm\bin"

if not defined COMPILER (
  for /f "delims=" %%I in ('where "%TOOL%" 2^>NUL') do if not defined COMPILER set "COMPILER=%%~fI"
)

if not defined COMPILER call :probe "%ProgramFiles%\LLVM\bin"
if not defined COMPILER call :probe "%ProgramFiles(x86)%\LLVM\bin"

for %%V in (2022 2019) do (
  for %%E in (Community Professional Enterprise BuildTools Preview) do (
    if not defined COMPILER call :probe "%ProgramFiles%\Microsoft Visual Studio\%%V\%%E\VC\Tools\Llvm\bin"
    if not defined COMPILER call :probe "%ProgramFiles(x86)%\Microsoft Visual Studio\%%V\%%E\VC\Tools\Llvm\bin"
  )
)

if not defined COMPILER (
  echo EA Code could not locate %TOOL%. Install LLVM or Visual Studio C++ Clang tools, or set EA_CODE_LLVM_BIN to the LLVM bin directory. 1>&2
  exit /b 1
)

"%COMPILER%" %*
exit /b %ERRORLEVEL%

:probe
if not defined COMPILER if exist "%~1\%TOOL%" set "COMPILER=%~1\%TOOL%"
exit /b 0
