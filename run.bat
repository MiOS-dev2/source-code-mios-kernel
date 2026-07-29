@echo off
echo Starting QEMU with MiOS.iso...
"C:\Program Files\qemu\qemu-system-x86_64.exe" -drive format=raw,file="MiOS.iso" -monitor stdio
pause