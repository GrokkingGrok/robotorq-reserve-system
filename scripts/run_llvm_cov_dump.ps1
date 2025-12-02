$llvm = 'C:\\Users\\Jon\\.rustup\\toolchains\\stable-x86_64-pc-windows-msvc\\lib\\rustlib\\x86_64-pc-windows-msvc\\bin\\llvm-cov.exe'
$prof = 'C:\\Users\\Jon\\Documents\\Project-Asimov\\robotorq-reserve-system\\target\\llvm-cov-target\\robotorq-reserve-system.profdata'
$objs = Get-ChildItem -Path C:\\Users\\Jon\\Documents\\Project-Asimov\\robotorq-reserve-system\\target\\llvm-cov-target -Filter *.exe -Recurse | ForEach-Object { $_.FullName }
 $args = @('show', '-instr-profile', $prof)
foreach ($o in $objs) { $args += '--object'; $args += $o }
$args += '--dump'
Write-Output "Running llvm-cov with $#($objs.Count) objects; output will be written to target\\llvm-cov-dump.txt"
& $llvm @args 2>&1 | Out-File -FilePath .\target\llvm-cov-dump.txt -Encoding utf8
Write-Output "Dump written to target\\llvm-cov-dump.txt"
Get-Content .\target\llvm-cov-dump.txt | Select-String -Pattern 'hash-mismatch','No profile record','mismatched data' -AllMatches | ForEach-Object { $_.Line }
