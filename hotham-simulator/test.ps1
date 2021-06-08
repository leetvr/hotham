Write-Output "All your OpenXR are belong to crab"
${Env:RUST_BACKTRACE} = 3
cargo build

try {
    Push-Location -Path "C:\Users\kanem\Development\OpenXR-SDK-Source\build\src\tests\hello_xr\Debug"
    .\hello_xr.exe -g Vulkan2
}
catch {
    Write-Warning "Problem!"
}
finally {
    Pop-Location
}