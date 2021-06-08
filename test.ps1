Write-Output "CUBES"
${env:RUST_BACKTRACE} = 1

try {
    Invoke-Command -ErrorAction Stop -ScriptBlock { cargo build -p hotham-simulator }
    cargo run -p hotham-cubeworld
}
catch {
    Write-Warning "Problem!"
}