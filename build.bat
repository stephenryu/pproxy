@echo off
setlocal enabledelayedexpansion

REM Build pproxy with embedded build metadata from build.rs.

cd /d "%~dp0"

set "EXITCODE=0"
echo Building pproxy

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
	exit /b %EXITCODE%
)

