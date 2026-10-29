# Writes the CHANGELOG.md section for a release tag (`v1.2.3` -> `## [1.2.3] ...`) to -Out.
param([Parameter(Mandatory)] [string]$Tag, [Parameter(Mandatory)] [string]$Out)
$ErrorActionPreference = 'Stop'

$version = [regex]::Escape($Tag.TrimStart('v'))
$lines = Get-Content CHANGELOG.md
$start = $null
for ($i = 0; $i -lt $lines.Count; $i++) {
  if ($lines[$i] -match "^## \[$version\]") { $start = $i + 1; break }
}
if ($null -eq $start) { throw "CHANGELOG.md has no section for $Tag" }

$end = $lines.Count
for ($i = $start; $i -lt $lines.Count; $i++) {
  # The next version heading, or the link references at the bottom.
  if ($lines[$i] -match '^## \[' -or $lines[$i] -match '^\[[^\]]+\]: ') { $end = $i; break }
}
$notes = ($lines[$start..($end - 1)] -join "`n").Trim()
if (-not $notes) { throw "CHANGELOG.md section for $Tag is empty" }
Set-Content -Path $Out -Value $notes -Encoding utf8NoBOM
