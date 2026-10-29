# Fails unless every version in the repo equals the release tag (`v1.2.3` -> `1.2.3`).
param([Parameter(Mandatory)] [string]$Tag)
$ErrorActionPreference = 'Stop'

$want = $Tag.TrimStart('v')
$found = [ordered]@{
  'apps/panel/src-tauri/tauri.conf.json' = (Get-Content apps/panel/src-tauri/tauri.conf.json -Raw | ConvertFrom-Json).version
  'apps/panel/package.json'              = (Get-Content apps/panel/package.json -Raw | ConvertFrom-Json).version
  'packages/touch-kit-web/package.json'  = (Get-Content packages/touch-kit-web/package.json -Raw | ConvertFrom-Json).version
  'bindings/dotnet/TouchKit/TouchKit.csproj' = ([xml](Get-Content bindings/dotnet/TouchKit/TouchKit.csproj -Raw)).Project.PropertyGroup.Version
}
foreach ($toml in 'apps/panel/src-tauri/Cargo.toml', 'crates/touch-core/Cargo.toml', 'crates/touch-ffi/Cargo.toml', 'crates/touch-kit-tauri/Cargo.toml', 'touchprobe/Cargo.toml') {
  $found[$toml] = ((Select-String -Path $toml -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches.Groups[1].Value)
}

$wrong = $found.GetEnumerator() | Where-Object { $_.Value -ne $want }
if ($wrong) {
  $wrong | ForEach-Object { Write-Host "$($_.Key): $($_.Value)" }
  throw "versions above don't match tag $Tag"
}
Write-Host "all versions are $want"
