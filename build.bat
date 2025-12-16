@echo off
setlocal enabledelayedexpansion

REM Build pproxy with embedded commit/build timestamp.
REM These are read at compile time via option_env!("PPROXY_COMMIT") and option_env!("PPROXY_BUILD_UNIX").

cd /d "%~dp0"

set "EXITCODE=0"

set "PPROXY_COMMIT="
for /f "usebackq delims=" %%i in (`git rev-parse --short HEAD 2^>NUL`) do set "PPROXY_COMMIT=%%i"
if "%PPROXY_COMMIT%"=="" set "PPROXY_COMMIT=unknown"

set "PPROXY_BUILD_UNIX="
for /f "usebackq delims=" %%i in (`powershell -NoProfile -Command "[DateTimeOffset]::UtcNow.ToUnixTimeSeconds()"`) do set "PPROXY_BUILD_UNIX=%%i"
if "%PPROXY_BUILD_UNIX%"=="" set "PPROXY_BUILD_UNIX=0"

echo Building pproxy (commit=%PPROXY_COMMIT%, build_unix=%PPROXY_BUILD_UNIX%)

REM On Windows, a running pproxy.exe can lock target\release\pproxy.exe and break builds.
taskkill /IM pproxy.exe /F >NUL 2>&1

cargo build --release
if errorlevel 1 (
	set "EXITCODE=%errorlevel%"
	goto :cleanup
)

if exist "pproxy.yaml" (
	copy /y "pproxy.yaml" "target\release\pproxy.yaml" >NUL
)

echo Done: target\release\pproxy.exe

:cleanup
endlocal & (
	REM Clear variables in the caller environment.
	set "PPROXY_COMMIT="
	set "PPROXY_BUILD_UNIX="
	exit /b %EXITCODE%
)

