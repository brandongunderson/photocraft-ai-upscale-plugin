param([switch]$VerifyOnly)
$ErrorActionPreference = 'Stop'
$pluginRoot = [IO.Path]::GetFullPath((Split-Path $PSScriptRoot))
$runtimeRoot = Join-Path $pluginRoot 'runtime'
$runtimePrefix = [IO.Path]::GetFullPath($runtimeRoot).TrimEnd('\') + '\'
$manifest = Get-Content -LiteralPath (Join-Path $runtimeRoot 'downloads.json') -Raw | ConvertFrom-Json

foreach ($asset in $manifest.files | Where-Object { $_.default }) {
    $destination = [IO.Path]::GetFullPath((Join-Path $runtimeRoot $asset.path))
    if (-not $destination.StartsWith($runtimePrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'An asset path escapes the runtime directory.'
    }
    if ($asset.sha256 -notmatch '^[0-9a-f]{64}$' -or ([Uri]$asset.url).Scheme -ne 'https') {
        throw "Invalid pinned asset entry: $($asset.path)"
    }
    if ((Test-Path -LiteralPath $destination -PathType Leaf) -and
        (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant() -eq $asset.sha256) {
        Write-Output "Verified $($asset.path)"
        continue
    }
    if ($VerifyOnly) { throw "Missing or modified runtime asset: $($asset.path)" }
    New-Item -ItemType Directory -Path (Split-Path $destination) -Force | Out-Null
    $temporary = $destination + '.' + [Guid]::NewGuid().ToString('N') + '.part'
    try {
        Invoke-WebRequest -Uri $asset.url -OutFile $temporary -UserAgent 'Photocraft-dev'
        if ((Get-FileHash -LiteralPath $temporary -Algorithm SHA256).Hash.ToLowerInvariant() -ne $asset.sha256) {
            throw "Checksum mismatch: $($asset.path)"
        }
        Move-Item -LiteralPath $temporary -Destination $destination -Force
        Write-Output "Downloaded and verified $($asset.path)"
    } finally {
        if (Test-Path -LiteralPath $temporary -PathType Leaf) { Remove-Item -LiteralPath $temporary -Force }
    }
}
