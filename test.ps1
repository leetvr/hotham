${env:RUST_BACKTRACE} = 1

try {
    Invoke-Command -ErrorAction Stop -ScriptBlock { cargo build -p hotham-simulator }
    C:\Users\kanem\Development\OpenXR-SDK-Source\build\src\tests\hello_xr\Debug\hello_xr.exe -v -g Vulkan2 -ff Hmd --space Stage -bm Opaque -vc Stereo > test.log
}
catch {
    Write-Warning "Problem!"
}