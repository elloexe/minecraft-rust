@echo off
REM Launch Claudium with your Microsoft sign-in configured.
REM Put your client ID (one line, no quotes) in client-id.txt next to this file.
setlocal
set "HERE=%~dp0"
if not exist "%HERE%client-id.txt" (
  echo Create %HERE%client-id.txt with your CLAUDIUM_MSA_CLIENT_ID on one line.
  pause
  exit /b 1
)
set /p CLAUDIUM_MSA_CLIENT_ID=<"%HERE%client-id.txt"
start "" "%HERE%Claudium-Launcher.exe" %*
endlocal
