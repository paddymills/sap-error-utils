cargo build --release --all-features
Copy-Item .\target\release\inbox_errors.exe '\\hssfileserv1\shops\inventory\sap'

if ($env:USERNAME == "PMiller1") {
    Copy-Item .\target\release\inbox_errors.exe "$env:USERPROFILE\src\cogi\inbox"
}
