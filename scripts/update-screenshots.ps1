$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
$previousPage = $env:QBIT_SMOKE_PAGE
$previousZoom = $env:QBIT_SMOKE_ZOOM
$previousTray = $env:QBIT_SMOKE_TRAY
$previousColumns = $env:QBIT_SMOKE_COLUMNS
$previousTheme = $env:QBIT_SMOKE_THEME
$previousMulti = $env:QBIT_SMOKE_MULTI
$previousSpace = $env:QBIT_SMOKE_FREE_SPACE
$previousAddHeight = $env:QBIT_SMOKE_ADD_HEIGHT
$previousAddMany = $env:QBIT_SMOKE_ADD_MANY
try {
    Remove-Item Env:QBIT_SMOKE_TRAY, Env:QBIT_SMOKE_COLUMNS, Env:QBIT_SMOKE_MULTI, Env:QBIT_SMOKE_FREE_SPACE, Env:QBIT_SMOKE_ADD_HEIGHT, Env:QBIT_SMOKE_ADD_MANY -ErrorAction SilentlyContinue
    cargo build --locked --features ui-smoke
    if ($LASTEXITCODE -ne 0) { throw 'Screenshot build failed' }
    $version = [regex]::Match((Get-Content -Raw Cargo.toml), '(?m)^version = "(\d+\.\d+\.\d+)"').Groups[1].Value
    if (-not $version) { throw 'Application version missing from Cargo.toml' }
    $gallery = "docs/screenshots/v$version"
    New-Item -ItemType Directory -Path $gallery -Force | Out-Null
    $env:QBIT_SMOKE_ZOOM = '1'
    $env:QBIT_SMOKE_THEME = 'marine'
    foreach ($page in @('torrents', 'files', 'speed')) {
        $env:QBIT_SMOKE_PAGE = $page
        Remove-Item -LiteralPath dist/ui-smoke.png -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath dist/ui-smoke-state.ron -ErrorAction SilentlyContinue
        & ./target/debug/qbit-remote.exe
        if ($LASTEXITCODE -ne 0) { throw "Screenshot failed: $page" }
        Copy-Item -LiteralPath dist/ui-smoke.png -Destination (Join-Path $gallery "$page.png")
    }
    $env:QBIT_SMOKE_THEME = 'graphite'
    $env:QBIT_SMOKE_PAGE = 'server-tabs'
    Remove-Item -LiteralPath dist/ui-smoke.png -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath dist/ui-smoke-state.ron -ErrorAction SilentlyContinue
    & ./target/debug/qbit-remote.exe
    if ($LASTEXITCODE -ne 0) { throw 'Screenshot failed: graphite' }
    Copy-Item -LiteralPath dist/ui-smoke.png -Destination (Join-Path $gallery 'graphite.png')
    $env:QBIT_SMOKE_THEME = 'white'
    $env:QBIT_SMOKE_PAGE = 'files'
    Remove-Item -LiteralPath dist/ui-smoke.png -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath dist/ui-smoke-state.ron -ErrorAction SilentlyContinue
    & ./target/debug/qbit-remote.exe
    if ($LASTEXITCODE -ne 0) { throw 'Screenshot failed: white' }
    Copy-Item -LiteralPath dist/ui-smoke.png -Destination (Join-Path $gallery 'white.png')
    $env:QBIT_SMOKE_THEME = 'graphite'
    $env:QBIT_SMOKE_MULTI = '1'
    $env:QBIT_SMOKE_PAGE = 'actions'
    Remove-Item -LiteralPath dist/ui-smoke.png -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath dist/ui-smoke-state.ron -ErrorAction SilentlyContinue
    & ./target/debug/qbit-remote.exe
    if ($LASTEXITCODE -ne 0) { throw 'Screenshot failed: actions' }
    Copy-Item -LiteralPath dist/ui-smoke.png -Destination (Join-Path $gallery 'actions.png')
    $env:QBIT_SMOKE_PAGE = 'torrents'
    Remove-Item -LiteralPath dist/ui-smoke.png -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath dist/ui-smoke-state.ron -ErrorAction SilentlyContinue
    & ./target/debug/qbit-remote.exe
    if ($LASTEXITCODE -ne 0) { throw 'Screenshot failed: multiselect' }
    Copy-Item -LiteralPath dist/ui-smoke.png -Destination (Join-Path $gallery 'multiselect.png')
    Remove-Item Env:QBIT_SMOKE_MULTI, Env:QBIT_SMOKE_FREE_SPACE, Env:QBIT_SMOKE_ADD_HEIGHT, Env:QBIT_SMOKE_ADD_MANY -ErrorAction SilentlyContinue
    $env:QBIT_SMOKE_PAGE = 'rss-import'
    Remove-Item -LiteralPath dist/ui-smoke.png -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath dist/ui-smoke-state.ron -ErrorAction SilentlyContinue
    & ./target/debug/qbit-remote.exe
    if ($LASTEXITCODE -ne 0) { throw 'Screenshot failed: rss-import' }
    Copy-Item -LiteralPath dist/ui-smoke.png -Destination (Join-Path $gallery 'rss-import.png')
    $env:QBIT_SMOKE_PAGE = 'add-torrent'
    Remove-Item -LiteralPath dist/ui-smoke.png, dist/ui-smoke-state.ron -ErrorAction SilentlyContinue
    & ./target/debug/qbit-remote.exe
    if ($LASTEXITCODE -ne 0) { throw 'Screenshot failed: add-torrent' }
    Copy-Item -LiteralPath dist/ui-smoke.png -Destination (Join-Path $gallery 'add-torrent.png')
    foreach ($entry in @(
        @{ Page = 'settings'; File = 'settings.png' },
        @{ Page = 'general-layout'; File = 'general.png' },
        @{ Page = 'graph-disabled'; File = 'graph-disabled.png' },
        @{ Page = 'console'; File = 'console.png' },
        @{ Page = 'peers'; File = 'peers.png' },
        @{ Page = 'trackers'; File = 'trackers.png'; Zoom = '0.8' },
        @{ Page = 'about'; File = 'about.png' },
        @{ Page = 'associations'; File = 'file-associations.png' },
        @{ Page = 'pieces'; File = 'pieces.png' },
        @{ Page = 'torrent-graph'; File = 'torrent-graph.png' },
        @{ Page = 'actions-bottom'; File = 'actions-bottom.png' },
        @{ Page = 'path-mappings'; File = 'path-mappings.png' },
        @{ Page = 'actions'; File = 'torrent-open-menu.png' },
        @{ Page = 'torrents'; File = 'status-bar.png' }
    )) {
        $env:QBIT_SMOKE_PAGE = $entry.Page
        $env:QBIT_SMOKE_ZOOM = if ($entry.Zoom) { $entry.Zoom } else { '1' }
        Remove-Item -LiteralPath dist/ui-smoke.png, dist/ui-smoke-state.ron -ErrorAction SilentlyContinue
        & ./target/debug/qbit-remote.exe
        if ($LASTEXITCODE -ne 0) { throw "Screenshot failed: $($entry.Page)" }
        Copy-Item -LiteralPath dist/ui-smoke.png -Destination (Join-Path $gallery $entry.File)
    }
} finally {
    $env:QBIT_SMOKE_MULTI = $previousMulti
    $env:QBIT_SMOKE_FREE_SPACE = $previousSpace
    $env:QBIT_SMOKE_ADD_HEIGHT = $previousAddHeight
    $env:QBIT_SMOKE_ADD_MANY = $previousAddMany
    $env:QBIT_SMOKE_PAGE = $previousPage
    $env:QBIT_SMOKE_ZOOM = $previousZoom
    $env:QBIT_SMOKE_TRAY = $previousTray
    $env:QBIT_SMOKE_COLUMNS = $previousColumns
    $env:QBIT_SMOKE_THEME = $previousTheme
    Pop-Location
}






