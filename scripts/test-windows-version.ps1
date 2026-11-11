param([Parameter(Mandatory = $true)][string]$Executable)

$ErrorActionPreference = 'Stop'
$manifest = Get-Content (Join-Path $PSScriptRoot '../Cargo.toml') -Raw
$version = [regex]::Match($manifest, '(?m)^version = "([^"]+)"').Groups[1].Value
if (-not $version) { throw 'No package version found in Cargo.toml' }
$info = [Diagnostics.ProcessStartInfo]::new()
$info.FileName = (Resolve-Path $Executable).Path
$info.Arguments = '--version'
$info.UseShellExecute = $false
$info.RedirectStandardOutput = $true
$info.RedirectStandardError = $true
$process = [Diagnostics.Process]::new()
$process.StartInfo = $info
$started = $false
try {
    [void]$process.Start()
    $started = $true
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $stderr = $process.StandardError.ReadToEndAsync()
    if (-not $process.WaitForExit(10000)) { throw 'Version check timed out' }
    if ($process.ExitCode -ne 0 -or $stdout.Result.Trim() -ne "meatshell $version") {
        throw "Version check failed (exit $($process.ExitCode)): $($stdout.Result) $($stderr.Result)"
    }
    Write-Output $stdout.Result.Trim()
} finally {
    if ($started -and -not $process.HasExited) { $process.Kill(); $process.WaitForExit() }
    $process.Dispose()
}
