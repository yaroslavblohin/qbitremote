$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
$previousPage = $env:QBIT_SMOKE_PAGE
$previousZoom = $env:QBIT_SMOKE_ZOOM
$previousTray = $env:QBIT_SMOKE_TRAY
$previousColumns = $env:QBIT_SMOKE_COLUMNS
$previousTheme = $env:QBIT_SMOKE_THEME
$previousMulti = $env:QBIT_SMOKE_MULTI
try {
    Remove-Item Env:QBIT_SMOKE_TRAY, Env:QBIT_SMOKE_COLUMNS, Env:QBIT_SMOKE_MULTI -ErrorAction SilentlyContinue
    cargo build --locked --features ui-smoke
    if ($LASTEXITCODE -ne 0) { throw 'Screenshot build failed' }
    New-Item -ItemType Directory -Path docs/screenshots -Force | Out-Null
    $env:QBIT_SMOKE_ZOOM = '1'
    $env:QBIT_SMOKE_THEME = 'marine'
    foreach ($page in @('torrents', 'files', 'speed')) {
        $env:QBIT_SMOKE_PAGE = $page
        Remove-Item -LiteralPath dist/ui-smoke.png -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath dist/ui-smoke-state.ron -ErrorAction SilentlyContinue
        & ./target/debug/qbit-remote.exe
        if ($LASTEXITCODE -ne 0) { throw "Screenshot failed: $page" }
        Copy-Item -LiteralPath dist/ui-smoke.png -Destination "docs/screenshots/$page.png"
    }
    $env:QBIT_SMOKE_THEME = 'graphite'
    $env:QBIT_SMOKE_PAGE = 'files'
    Remove-Item -LiteralPath dist/ui-smoke.png -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath dist/ui-smoke-state.ron -ErrorAction SilentlyContinue
    & ./target/debug/qbit-remote.exe
    if ($LASTEXITCODE -ne 0) { throw 'Screenshot failed: graphite' }
    Copy-Item -LiteralPath dist/ui-smoke.png -Destination docs/screenshots/graphite.png
    $env:QBIT_SMOKE_THEME = 'white'
    Remove-Item -LiteralPath dist/ui-smoke.png -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath dist/ui-smoke-state.ron -ErrorAction SilentlyContinue
    & ./target/debug/qbit-remote.exe
    if ($LASTEXITCODE -ne 0) { throw 'Screenshot failed: white' }
    Copy-Item -LiteralPath dist/ui-smoke.png -Destination docs/screenshots/white.png
    $env:QBIT_SMOKE_THEME = 'graphite'
    $env:QBIT_SMOKE_MULTI = '1'
    $env:QBIT_SMOKE_PAGE = 'actions'
    Remove-Item -LiteralPath dist/ui-smoke.png -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath dist/ui-smoke-state.ron -ErrorAction SilentlyContinue
    & ./target/debug/qbit-remote.exe
    if ($LASTEXITCODE -ne 0) { throw 'Screenshot failed: actions' }
    Copy-Item -LiteralPath dist/ui-smoke.png -Destination docs/screenshots/actions.png
    $env:QBIT_SMOKE_PAGE = 'torrents'
    Remove-Item -LiteralPath dist/ui-smoke.png -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath dist/ui-smoke-state.ron -ErrorAction SilentlyContinue
    & ./target/debug/qbit-remote.exe
    if ($LASTEXITCODE -ne 0) { throw 'Screenshot failed: multiselect' }
    Copy-Item -LiteralPath dist/ui-smoke.png -Destination docs/screenshots/multiselect.png
    Remove-Item Env:QBIT_SMOKE_MULTI -ErrorAction SilentlyContinue
    $env:QBIT_SMOKE_PAGE = 'rss-import'
    Remove-Item -LiteralPath dist/ui-smoke.png -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath dist/ui-smoke-state.ron -ErrorAction SilentlyContinue
    & ./target/debug/qbit-remote.exe
    if ($LASTEXITCODE -ne 0) { throw 'Screenshot failed: rss-import' }
    Copy-Item -LiteralPath dist/ui-smoke.png -Destination docs/screenshots/rss-import.png
} finally {
    $env:QBIT_SMOKE_MULTI = $previousMulti
    $env:QBIT_SMOKE_PAGE = $previousPage
    $env:QBIT_SMOKE_ZOOM = $previousZoom
    $env:QBIT_SMOKE_TRAY = $previousTray
    $env:QBIT_SMOKE_COLUMNS = $previousColumns
    $env:QBIT_SMOKE_THEME = $previousTheme
    Pop-Location
}



