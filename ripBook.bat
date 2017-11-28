echo OFF
SETLOCAL ENABLEEXTENSIONS ENABLEDELAYEDEXPANSION

REM call :Test
REM call :CleanAllFileNames C:\Users\vcase\AudioBooks\delme\book
REM goto :EOF

set /p BookName="Book Name: "
set /p AuthorName="Author: "
set /p CDNumStart="CD Start Index: "
set /p CDNumEnd="CD Last Index: "

set BookPath=%HOMEPATH%\AudioBooks\%AuthorName%\%BookName%
mkdir %BookPath%

FOR /L %%N IN (%CDNumStart%, 1, %CDNumEnd%) DO (
  ECHO\
  ECHO\
  echo Please insert CD# %%N and press any key to continue...
  pause
  REM You need the id3 if you are to uncomment the following line ... don't :)
  REM id3 -l "!BookName!" -a "!AuthorName!" -n %%N "!FileName!"
  call :ProcessCD %%N
  call :EjectCD
  ECHO Completed CD# %%N !
)
goto :DONE



:ProcessCD
SET CdIndex=00%1
SET CdIndex=%CdIndex:~-2%
SET CdOutputPath=%BookPath%\wip_cd_%CdIndex%
rmdir /S /Q %CdOutputPath% 2>nul
mkdir %CdOutputPath%
ECHO Ripping CD# %1 to temp dir %CdOutputPath% ...
"C:\Program Files (x86)\freac\freaccmd" -t 600 -track all -d "!CdOutputPath!"
call :NormalizeTrackNamesAll %CdIndex% %CdOutputPath%
ECHO Installing Ripped tracks to %BookPath%...
move %CdOutputPath%\* %BookPath%
rmdir /S /Q %CdOutputPath%
goto :EOF


:NormalizeTrackNamesAll
setlocal ENABLEDELAYEDEXPANSION 
SET CDNUM=%1
SET CDPATH=%2
SET /a COUNT=0
pushd %CDPATH%
call :CleanAllFileNames %CDPATH%
for /f "delims=" %%i in ('dir /B /o:d /A:-D') do (
  set /a COUNT += 1
  call :NormalizeTrackName %CDNUM% %CDPATH% "%%i" !COUNT!
)
popd
goto :EOF



:NormalizeTrackName
SET CDNUM=0%1
SET CDNUM=%CDNUM:~-2%
SET CDPATH=%2
SET TNAME=%3
set TINDEX=0%4%
set TINDEX=%TINDEX:~-2%
set NewName=cd%CDNUM%_t%TINDEX%.mp3
ren %TNAME% %NewName% 
goto :EOF


:EjectCD
echo Ejecting CD
@echo off
echo Set oWMP = CreateObject("WMPlayer.OCX.7")  >> %temp%\temp.vbs
echo Set colCDROMs = oWMP.cdromCollection       >> %temp%\temp.vbs
echo For i = 0 to colCDROMs.Count-1             >> %temp%\temp.vbs
echo colCDROMs.Item(i).Eject                    >> %temp%\temp.vbs
echo next                                       >> %temp%\temp.vbs
echo oWMP.close                                 >> %temp%\temp.vbs
%temp%\temp.vbs
timeout /t 1
del %temp%\temp.vbs
goto :EOF





:Test
set BookName=book
set AuthorName=delme
set CDNumStart=1
set CDNumEnd=1

set BookPath=%HOMEPATH%\AudioBooks\%AuthorName%\%BookName%
mkdir %BookPath%

call :NormalizeTrackNamesAll 1 %BookPath%
goto :EOF


:CleanAllFileNames
setlocal DISABLEDELAYEDEXPANSION 
set DIRNAME=%1
pushd %DIRNAME%

for /F "delims=" %%i in ('dir /b /A:-D %DIRNAME%') do (
  call :CleanFileName "%%i"
)
goto :EOF


:CleanFileName
  set f1=%1
  set f2=%f1:!=%
  ren %f1% %f2%
goto :EOF



:DONE
ECHO\
ECHO DONE!
goto :EOF


