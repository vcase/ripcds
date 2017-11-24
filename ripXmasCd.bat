echo OFF
SETLOCAL ENABLEEXTENSIONS ENABLEDELAYEDEXPANSION


set /p AlbumName="Album Name: "
set /p DriveIndex="Drive Index (0..N): "

set AlbumPath=%HOMEPATH%\Music\Christmas\%AlbumName%
rmdir /S /Q %AlbumPath%
mkdir %AlbumPath%

ECHO Ripping CD in drive %DriveIndex% to folder !AlbumPath!...
"C:\Program Files (x86)\freac\freaccmd" -cddb -t 600 -cd %DriveIndex% -track all -d "!AlbumPath!"

