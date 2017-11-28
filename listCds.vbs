REM To output to the console,
REM run as cscript.exe listCds.vbs
REM 
Set oWMP = CreateObject("WMPlayer.OCX.7")
Set colCDROMs = oWMP.cdromCollection
For i = 0 to colCDROMs.Count-1
	WScript.Echo "Drive index " & i & " = " & colCDROMs.Item(i).driveSpecifier
next
oWMP.close
