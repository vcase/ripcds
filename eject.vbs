REM To output to the console,
REM run as cscript.exe eject.vbs [DriveIndex]
IDRIVE = WScript.Arguments(0)
Set oWMP = CreateObject("WMPlayer.OCX.7")
Set colCDROMs = oWMP.cdromCollection
WScript.Echo "Drive index " & IDRIVE & " = " & colCDROMs.Item(IDRIVE).driveSpecifier
colCDROMs.Item(IDRIVE).Eject
oWM.close
