@echo off
SETLOCAL ENABLEEXTENSIONS ENABLEDELAYEDEXPANSION

REM Uncomment for quick testing
REM call :Test
REM call :CleanAllFileNames "%USERPROFILE%\AudioBooks\delme\book"
REM goto :EOF

set /p BookName="Book Name: "
set /p AuthorName="Author: "
set /p CDNumStart="CD Start Index: "
set /p CDNumEnd="CD Last Index: "

set "BookPath=%USERPROFILE%\AudioBooks\%AuthorName%\%BookName%"
if not exist "%BookPath%" mkdir "%BookPath%"

:: --- Verify that at least one CD/DVD drive exists ---
echo Checking for CD/DVD drives...
powershell -NoProfile -Command ^
  "if (-not (Get-CimInstance Win32_CDROMDrive)) { exit 1 }"
if errorlevel 1 (
  echo ERROR: No CD/DVD drive detected. Please connect or power on your drive.
  pause
  exit /b
)

:: --- Optional: display detected drive(s) ---
powershell -NoProfile -Command ^
  "Get-CimInstance Win32_CDROMDrive | Select-Object Drive, Name | Format-Table -AutoSize"

for /L %%N in (%CDNumStart%,1,%CDNumEnd%) do (
  echo.
  echo Please insert CD# %%N and press any key to continue...
  pause >nul
  call :ProcessCD %%N
  call :EjectCD
  echo Completed CD# %%N !
)
goto :DONE


:ProcessCD
set "CdIndex=00%~1"
set "CdIndex=%CdIndex:~-2%"
set "CdOutputPath=%BookPath%\wip_cd_%CdIndex%"

rmdir /S /Q "%CdOutputPath%" 2>nul
mkdir "%CdOutputPath%"

echo Ripping CD# %~1 to temp dir "%CdOutputPath%" ...
"C:\Program Files\freac\freaccmd.exe" -t 600 -track all -d "%CdOutputPath%"
if errorlevel 1 (
  echo ERROR: freaccmd failed.
  pause
)

call :NormalizeTrackNamesAll "%CdIndex%" "%CdOutputPath%"
echo Installing ripped tracks to "%BookPath%"...
move "%CdOutputPath%\*" "%BookPath%" >nul
rmdir /S /Q "%CdOutputPath%"
goto :EOF


:NormalizeTrackNamesAll
setlocal ENABLEDELAYEDEXPANSION
set "CDNUM=%~1"
set "CDPATH=%~2"
set /a COUNT=0

pushd "%CDPATH%" || (echo ERROR: cannot access "%CDPATH%" & exit /b)
call :CleanAllFileNames "%CDPATH%"

for /f "delims=" %%i in ('dir /B /A:-D /O:D') do (
  set /a COUNT+=1
  call :NormalizeTrackName "%CDNUM%" "%CDPATH%" "%%~i" "!COUNT!"
)
popd
endlocal
goto :EOF


:NormalizeTrackName
setlocal
set "CDNUM=0%~1"
set "CDNUM=%CDNUM:~-2%"
set "CDPATH=%~2"
set "TNAME=%~3"
set "TINDEX=0%~4"
set "TINDEX=%TINDEX:~-2%"
set "NewName=cd%CDNUM%_t%TINDEX%.mp3"
ren "%CDPATH%\%TNAME%" "%NewName%" 2>nul
endlocal
goto :EOF


:EjectCD
echo Ejecting CD...
powershell -NoProfile -Command ^
  "$drives = Get-CimInstance Win32_CDROMDrive; " ^
  "foreach ($d in $drives) { " ^
  "  try { (New-Object -ComObject Shell.Application).Namespace(17).ParseName($d.Drive).InvokeVerb('Eject') } catch {} }"
timeout /t 1 /nobreak >nul
goto :EOF


:Test
set "BookName=book"
set "AuthorName=delme"
set "CDNumStart=1"
set "CDNumEnd=1"

set "BookPath=%USERPROFILE%\AudioBooks\%AuthorName%\%BookName%"
if not exist "%BookPath%" mkdir "%BookPath%"
call :NormalizeTrackNamesAll 1 "%BookPath%"
goto :EOF


:CleanAllFileNames
setlocal DISABLEDELAYEDEXPANSION
set "DIRNAME=%~1"
pushd "%DIRNAME%" || (echo ERROR: cannot access "%DIRNAME%" & exit /b)
for /F "delims=" %%i in ('dir /b /A:-D') do (
  call :CleanFileName "%%~i"
)
popd
endlocal
goto :EOF


:CleanFileName
setlocal
set "f1=%~1"
set "f2=%f1:!=%"
if not "%f1%"=="%f2%" ren "%f1%" "%f2%"
endlocal
goto :EOF


:DONE
echo.
echo DONE!
pause
goto :EOF
