$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    cargo build --locked --features ui-smoke
    if ($LASTEXITCODE -ne 0) { throw 'Smoke build failed' }
    New-Item -ItemType Directory -Path dist -Force | Out-Null
    foreach ($state in @('hidden', 'visible', 'minimized')) {
        $env:QBIT_SMOKE_TRAY = $state
        $process = Start-Process -FilePath '.\target\debug\qbit-remote.exe' -WorkingDirectory (Get-Location).Path -WindowStyle Hidden -RedirectStandardOutput "dist\tray-$state.log" -RedirectStandardError "dist\tray-$state-error.log" -PassThru
        if (-not $process.WaitForExit(20000)) {
            $process.Kill()
            throw "Tray test timed out: $state"
        }
        $log = Get-Content "dist\tray-$state.log" -Raw
        if ($process.ExitCode -ne 0 -or $log -notmatch 'Tray lifecycle: normal exit' -or $log -notmatch 'Tray lifecycle: save called' -or $log -notmatch 'Tray clicks: left toggles, right preserves visibility, double-click suppressed, minimized restored') {
            Get-Content "dist\tray-$state-error.log"
            throw "Tray test failed: $state"
        }
        Write-Output "PASS: $state (10 close/restore + 10 left-click toggle cycles, right/double-click checks, normal save and exit)"
    }
} finally {
    Remove-Item Env:\QBIT_SMOKE_TRAY -ErrorAction SilentlyContinue
    Pop-Location
}
