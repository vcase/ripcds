@ECHO OFF
SETLOCAL ENABLEEXTENSIONS ENABLEDELAYEDEXPANSION

set ZIPEXE="C:\Program Files\7-Zip\7z.exe"
cls

set /p BookName="Book Name:  "
set /p AuthorName="Author Name:  "

set BookPath=%HOMEPATH%\AudioBooks\%AuthorName%\%BookName%
if NOT EXIST %BookPath% (
  echo No Such Book Directory "%BookPath%"
  goto :EOF
)

Echo Creating Book Zipfile %BookName%.zip
set ZIPFILE=%BookPath%\..\%BookName%.zip
if exist %ZIPFILE% del %ZIPFILE% 
FOR /F %%N IN ('dir /B /o:N /A:-D %BookPath%') DO (
  ECHO Adding file %%N 
  timeout /t 1 2>&1 >nul
  %ZIPEXE% a %ZIPFILE% %BookPath%\%%N 2>&1 | findstr "System ERROR" 
  set ziperr=!errorlevel!
  if !ziperr! EQU 0 (
    echo FAILED to zip file "%%N". Trying again.
    timeout /t 4 2>&1 >nul
    %ZIPEXE% a %ZIPFILE% %BookPath%\%%N 2>&1 | findstr "System ERROR" 
    set ziperr=!errorlevel!
    if !ziperr! EQU 0 (
      echo FAILED to zip file "%%N" again
      goto :EOF
    )
  )
)
Echo Created Book Zipfile %ZIPFILE%
goto :DONE



:DONE
ECHO\
ECHO DONE!
goto :EOF


