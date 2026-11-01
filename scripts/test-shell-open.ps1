param(
    [string]$LocalRoot,
    [string]$ServerRoot = '/downloads',
    [string]$RelativeFolder
)
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
$shell = New-Object -ComObject Shell.Application
try {
    foreach ($mode in @('torrent', 'tree')) {
        $temporary = [string]::IsNullOrEmpty($LocalRoot)
        if ($temporary) {
            $fixture = Join-Path (Get-Location).Path ('dist\shell-open-' + [Guid]::NewGuid().ToString('N'))
            $relative = 'Папка с пробелами & тест'
            $folder = Join-Path $fixture $relative
            New-Item -ItemType Directory -Path $folder -Force | Out-Null
        } else {
            if ([string]::IsNullOrEmpty($RelativeFolder)) { throw 'RelativeFolder is required with LocalRoot' }
            $fixture = $LocalRoot
            $relative = $RelativeFolder
            $folder = Join-Path $fixture $relative
            if (-not (Test-Path -LiteralPath $folder -PathType Container)) { throw "Folder unavailable: $folder" }
        }
        $existingWindows = @($shell.Windows() | ForEach-Object { $_.HWND })
        try {
            $env:QBIT_TEST_OPEN_PATH = $fixture
            $env:QBIT_TEST_OPEN_MODE = $mode
            $env:QBIT_TEST_OPEN_SERVER = $ServerRoot
            $env:QBIT_TEST_OPEN_RELATIVE = $relative
            cargo test --locked --all-features path_mapping::tests::native_shell_open_from_short_lived_worker -- --ignored --exact --nocapture
            if ($LASTEXITCODE -ne 0) { throw "Shell worker failed: $mode" }
            $deadline = [DateTime]::UtcNow.AddSeconds(20)
            $opened = $null
            do {
                foreach ($window in @($shell.Windows())) {
                    try {
                        if ($window.Document.Folder.Self.Path -eq $folder) { $opened = $window; break }
                    } catch { }
                }
                if ($null -eq $opened) { Start-Sleep -Milliseconds 200 }
            } while ($null -eq $opened -and [DateTime]::UtcNow -lt $deadline)
            if ($null -eq $opened) { throw "Explorer did not open the mapped folder: $mode" }
            Write-Output "PASS: $mode -> Explorer shows the mapped folder after the worker exits"
        } finally {
            # Close only the window showing our unique fixture, never other Explorer windows.
            foreach ($window in @($shell.Windows())) {
                try { if ($window.Document.Folder.Self.Path -eq $folder -and $window.HWND -notin $existingWindows) { $window.Quit() } } catch { }
            }
            if ($temporary) {
                Remove-Item -LiteralPath $folder -ErrorAction SilentlyContinue
                Remove-Item -LiteralPath $fixture -ErrorAction SilentlyContinue
            }
        }
    }
} finally {
    Remove-Item Env:\QBIT_TEST_OPEN_PATH,Env:\QBIT_TEST_OPEN_MODE,Env:\QBIT_TEST_OPEN_SERVER,Env:\QBIT_TEST_OPEN_RELATIVE -ErrorAction SilentlyContinue
    Pop-Location
}
