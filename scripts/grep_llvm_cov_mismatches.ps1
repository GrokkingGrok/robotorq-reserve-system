$llvm = "C:\\Users\\Jon\\.rustup\\toolchains\\stable-x86_64-pc-windows-msvc\\lib\\rustlib\\x86_64-pc-windows-msvc\\bin\\llvm-cov.exe"
$prof = "target\\llvm-cov-target\\robotorq-reserve-system.profdata"

Get-ChildItem -Path target\\llvm-cov-target\\debug -Filter *.exe -Recurse | ForEach-Object {
    $path = $_.FullName
    Write-Output "==== $path ===="
    & $llvm show -instr-profile=$prof -object $path --dump 2>&1 | Select-String -Pattern 'hash-mismatch|mismatched data|warning:' -AllMatches | ForEach-Object { $_.Line }
}

Write-Output "Done";
