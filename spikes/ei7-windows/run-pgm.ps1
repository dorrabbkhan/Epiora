# run-pgm.ps1 — run oswego-tables.pgm through Epi Info 7 Classic Analysis.
#
# Epiora v0.1 Task 3 spike. Locates a built (or installed) Analysis.exe,
# renders oswego-tables.pgm into a *.pgm7 file with absolute paths — the
# Analysis.exe command-line loader only honours arguments whose file name
# contains ".pgm7" — then runs it. The program's own QUIT statement closes
# the application, so a timeout is a headless-run failure, not normal.
#
# Paths substituted into the program use forward slashes: inside .pgm
# BraceString and String literals a backslash escapes the next character
# (see Epi.Core.Interpreter grammar), so "D:\a" would become "D:a".
#
# Exit code 0 only if ei7-output contains a .html file afterwards.

[CmdletBinding()]
param(
    # Root to search for Analysis.exe (the EI7 source checkout whose
    # build output lands in <root>\build\<config>\).
    [string]$Ei7Dir = "ei7",
    # Directory that receives the ROUTEOUT html; asserted on by the workflow.
    [string]$OutputDir = "ei7-output",
    # Headless run guard: Analysis.exe is killed after this many seconds.
    [int]$TimeoutSeconds = 300
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$ei7 = Join-Path $repoRoot $Ei7Dir
$outDir = Join-Path $repoRoot $OutputDir

Write-Host "run-pgm: Ei7Dir=$ei7 OutputDir=$outDir Timeout=${TimeoutSeconds}s"

# --- 1. Locate Analysis.exe ---------------------------------------------------
$analysis = $env:EI7_ANALYSIS_EXE
if (-not $analysis) {
    $candidates = Get-ChildItem -Path $ei7 -Recurse -Filter "Analysis.exe" -File -ErrorAction SilentlyContinue |
        Sort-Object { if ($_.FullName -match 'release') { 0 } else { 1 } }, FullName
    $analysis = $candidates | Select-Object -First 1 -ExpandProperty FullName
}
if (-not $analysis -or -not (Test-Path $analysis)) {
    Write-Error "run-pgm: Analysis.exe not found under $ei7 (set EI7_ANALYSIS_EXE to point at an installed copy)"
    exit 1
}
$exeDir = Split-Path $analysis -Parent
Write-Host "run-pgm: Analysis.exe=$analysis"

# --- 2. Locate the Oswego sample project -------------------------------------
$samplePrj = Join-Path $exeDir "Projects\Sample\Sample.prj"
if (-not (Test-Path $samplePrj)) {
    $samplePrj = Get-ChildItem -Path $ei7 -Recurse -Filter "Sample.prj" -File -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match 'Sample' } |
        Select-Object -First 1 -ExpandProperty FullName
}
if (-not $samplePrj -or -not (Test-Path $samplePrj)) {
    Write-Error "run-pgm: Sample.prj (Oswego sample project) not found"
    exit 1
}
Write-Host "run-pgm: Sample.prj=$samplePrj"

# --- 3. Render the .pgm7 -------------------------------------------------------
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$outHtml = Join-Path $outDir "oswego-tables.html"
$pgmText = Get-Content (Join-Path $PSScriptRoot "oswego-tables.pgm") -Raw
$pgmText = $pgmText.Replace("@SAMPLE_PRJ@", ($samplePrj -replace '\\', '/'))
$pgmText = $pgmText.Replace("@OUTPUT_HTML@", ($outHtml -replace '\\', '/'))

$workDir = Join-Path $outDir "work"
New-Item -ItemType Directory -Force -Path $workDir | Out-Null
$pgm7 = Join-Path $workDir "oswego-tables.pgm7"
Set-Content -Path $pgm7 -Value $pgmText -Encoding ASCII
Write-Host "run-pgm: rendered $pgm7"
Get-Content $pgm7 | Write-Host

# --- 4. Run Analysis.exe and wait for QUIT ------------------------------------
$proc = Start-Process -FilePath $analysis -ArgumentList "`"$pgm7`"" `
    -WorkingDirectory $exeDir -PassThru
Write-Host "run-pgm: started PID $($proc.Id), waiting up to ${TimeoutSeconds}s"

if (-not $proc.WaitForExit($TimeoutSeconds * 1000)) {
    $proc.Kill()
    Write-Error "run-pgm: Analysis.exe did not exit within ${TimeoutSeconds}s (headless run failed)"
    exit 1
}
Write-Host "run-pgm: Analysis.exe exited with code $($proc.ExitCode)"

# --- 5. Verify output ----------------------------------------------------------
$deadline = (Get-Date).AddSeconds(15)
while (-not (Test-Path $outHtml) -and (Get-Date) -lt $deadline) {
    Start-Sleep -Milliseconds 500
}
if (-not (Test-Path $outHtml)) {
    Write-Error "run-pgm: no output at $outHtml — ROUTEOUT did not produce html"
    exit 1
}
Write-Host "run-pgm: OK — $outHtml ($((Get-Item $outHtml).Length) bytes)"
exit 0
