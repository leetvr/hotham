${env:RUST_BACKTRACE} = 1

# cargo build
# if ($?) {
#     # Set-Location ..\openxrs\openxr
#     # cargo run --example vulkan
#     cargo run -p hotham-cubeworld

#     # & "C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.EXE" --build c:/Users/kanem/Development/OpenXR-SDK-Source/build --config Debug --target hello_xr -j 18 --
#     # & "C:\Users\kanem\Development\OpenXR-SDK-Source\build\src\tests\hello_xr\Debug\hello_xr.exe" -v -g Vulkan2 -ff Hmd --space Stage -bm Opaque -vc Stereo
#     Write-Output "HelloXR exited with $LASTEXITCODE";
# }
# else {
#     Write-Warning "Hotham simulator failed to compile."
# }
Set-Location hotham-cubeworld
cargo apk run

if ($?) {
    adb logcat RustStdoutStderr:D *:S

}


Set-Location C:\Users\kanem\Development\hotham