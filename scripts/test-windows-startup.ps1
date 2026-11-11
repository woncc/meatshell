param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [int]$ObserveSeconds = 10
)

$ErrorActionPreference = 'Stop'
$dataDir = Join-Path ([IO.Path]::GetTempPath()) ('meatshell-startup-' + [Guid]::NewGuid())
$process = $null
$started = $false
try {
    New-Item -ItemType Directory $dataDir | Out-Null
    [IO.File]::WriteAllText((Join-Path $dataDir 'sessions.json'), '{"sessions":[],"update_check_disabled":true}')
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = (Resolve-Path $Executable).Path
    $info.Arguments = '--data-dir "' + $dataDir + '"'
    $info.WorkingDirectory = Split-Path $info.FileName
    $info.UseShellExecute = $false
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.EnvironmentVariables['RUST_BACKTRACE'] = 'full'
    $info.EnvironmentVariables['SLINT_BACKEND'] = 'winit-software'
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $info
    [void]$process.Start()
    $started = $true
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $stderr = $process.StandardError.ReadToEndAsync()
    if ($process.WaitForExit($ObserveSeconds * 1000)) {
        throw "GUI exited during startup (exit $($process.ExitCode)).`n$($stdout.Result)`n$($stderr.Result)"
    }
    $process.Refresh()
    if ($process.MainWindowHandle -eq [IntPtr]::Zero -or -not $process.Responding) {
        throw 'GUI did not create a responsive native window.'
    }
    Write-Output "Windows startup passed: responsive window, session $($process.SessionId), ${ObserveSeconds}s."
} finally {
    if ($started -and -not $process.HasExited) {
        [void]$process.CloseMainWindow()
        if (-not $process.WaitForExit(3000)) {
            $process.Kill()
            $process.WaitForExit()
        }
    }
    if ($process) { $process.Dispose() }
    if (Test-Path $dataDir) { Remove-Item -LiteralPath $dataDir -Recurse -Force }
}
