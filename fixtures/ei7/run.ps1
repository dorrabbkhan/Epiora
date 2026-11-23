# run.ps1 — run fixtures/ei7/oswego.pgm and every fixtures/ei7/v11/*.pgm
# through Epi Info 7 Classic Analysis.
#
# Epiora v0.2 Task 3. Promoted from spikes/ei7-windows/run-pgm.ps1 and
# narrowed to the single route the ei7.yml workflow uses: the official
# 7.2.7.0 Analysis.exe and the Sample.mdb inside the release ZIP (the
# .prj-view and source-built routes of the spike are not repeated here —
# see docs/spikes/ei7-windows.md for why .prj READs cannot work on CI).
# v1.1 Task 3 added the v11/ corpus loop.
#
# Locates Analysis.exe under -Ei7Dir (the extracted release ZIP) or honours
# EI7_ANALYSIS_EXE, renders each .pgm into a *.pgm7 file with absolute
# paths — Analysis.exe's command-line scan only accepts arguments whose
# file name contains ".pgm7" — then runs it. The program's own QUIT
# statement closes the application, so a timeout is a headless-run
# failure, not normal.
#
# Substitution tokens:
#   @SAMPLE_MDB@   Sample.mdb inside the release ZIP (oswego.pgm)
#   @WORK_DIR@     ei7-out/work/v11-data — holds SCRATCH COPIES of the
#                  committed fixture .mdb files, because Epi Info's MERGE
#                  writes into the destination table's file on disk; the
#                  committed fixtures must never be written.
#   @OUTPUT_HTML@  the program's ROUTEOUT target under -OutputDir.
#
# Paths substituted into programs use forward slashes: inside .pgm
# BraceString and String literals a backslash escapes the next character.
#
# Every program is attempted even if an earlier one fails; exit code 0 only
# if every expected html exists afterwards.

[CmdletBinding()]
param(
    # Root searched recursively for Analysis.exe (extracted release ZIP).
    [string]$Ei7Dir = "ei7-rel",
    # Directory that receives the html outputs; asserted on by the workflow.
    [string]$OutputDir = "ei7-out",
    # Headless run guard: Analysis.exe is killed after this many seconds.
    [int]$TimeoutSeconds = 300
)

$ErrorActionPreference = "Continue"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$ei7 = Join-Path $repoRoot $Ei7Dir
$outDir = Join-Path $repoRoot $OutputDir

Write-Host "run: Ei7Dir=$ei7 OutputDir=$outDir Timeout=${TimeoutSeconds}s"

# --- 1. Locate Analysis.exe -------------------------------------------------
$analysis = $env:EI7_ANALYSIS_EXE
if (-not ($analysis -and (Test-Path $analysis))) {
    $analysis = Get-ChildItem -Path $ei7 -Recurse -Filter "Analysis.exe" -File -ErrorAction SilentlyContinue |
        Select-Object -First 1 -ExpandProperty FullName
}
if (-not ($analysis -and (Test-Path $analysis))) {
    Write-Error "run: no Analysis.exe found under $ei7 and EI7_ANALYSIS_EXE not set"
    exit 1
}
$exeDir = Split-Path $analysis -Parent
Write-Host "run: Analysis.exe=$analysis"

# --- 2. Locate the release ZIP's Sample.mdb ---------------------------------
$mdb = Join-Path $exeDir "Projects\Sample\Sample.mdb"
if (-not (Test-Path $mdb)) {
    Write-Error "run: Sample.mdb not found at $mdb"
    exit 1
}
$mdbHash = (Get-FileHash $mdb -Algorithm SHA256).Hash.ToLower()
Write-Host "run: Sample.mdb=$mdb"
Write-Host "run: Sample.mdb SHA256=$mdbHash"

# --- 3. Prepare output and the v11 scratch-data directory --------------------
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$workDir = Join-Path $outDir "work"
New-Item -ItemType Directory -Force -Path $workDir | Out-Null
$v11Data = Join-Path $workDir "v11-data"
New-Item -ItemType Directory -Force -Path $v11Data | Out-Null
$v11Out = Join-Path $outDir "v11"
New-Item -ItemType Directory -Force -Path $v11Out | Out-Null

foreach ($fixture in @(
        "fixtures\legacy\samples\ei7\Sample\Sample.mdb",
        "fixtures\legacy\samples\ei7\Nutrition\Nutrition.mdb")) {
    $src = Join-Path $repoRoot $fixture
    if (-not (Test-Path $src)) {
        Write-Error "run: fixture $src not found"
        exit 1
    }
    Copy-Item $src $v11Data -Force
    $h = (Get-FileHash (Join-Path $v11Data (Split-Path $src -Leaf)) -Algorithm SHA256).Hash.ToLower()
    Write-Host "run: v11-data $(Split-Path $src -Leaf) SHA256=$h"
}

# --- 4. Diagnostics for a hung run ------------------------------------------
function Write-WindowDiagnostics {
    param([int]$ProcessId)
    try {
        Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
        $root = [System.Windows.Automation.AutomationElement]::RootElement
        $cond = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ProcessIdProperty, $ProcessId)
        $wins = $root.FindAll([System.Windows.Automation.TreeScope]::Children, $cond)
        Write-Host "run: $($wins.Count) top-level window(s) for PID $ProcessId"
        foreach ($w in $wins) {
            Write-Host ("  window: '{0}' [{1}]" -f $w.Current.Name, $w.Current.ClassName)
            $texts = $w.FindAll([System.Windows.Automation.TreeScope]::Descendants,
                [System.Windows.Automation.Condition]::TrueCondition)
            foreach ($t in $texts) {
                $name = $t.Current.Name
                if ($name) { Write-Host ("    {0}: {1}" -f $t.Current.ControlType.ProgrammaticName, $name) }
                if ($t.Current.ControlType.ProgrammaticName -match 'Document|Edit') {
                    try {
                        $tp = $t.GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern)
                        $txt = $tp.DocumentRange.GetText(4000)
                        if ($txt -and $txt.Trim()) {
                            Write-Host "    --- control text ---"
                            $txt | Write-Host
                        }
                    } catch {}
                }
            }
        }
    } catch { Write-Host "run: UIA dump failed: $_" }
    foreach ($sub in "Configuration", "Output", "Logs") {
        $d = Join-Path $exeDir $sub
        if (Test-Path $d) {
            Write-Host "run: $d"
            Get-ChildItem $d -Recurse -File | ForEach-Object {
                Write-Host ("    {0} ({1} bytes, {2})" -f $_.FullName, $_.Length, $_.LastWriteTime)
            }
        }
    }
    Get-ChildItem -Path $exeDir -Recurse -Filter "output*.html" -File -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 2 | ForEach-Object {
            Write-Host "run: --- $($_.FullName) ---"
            Get-Content $_.FullName | ForEach-Object { $_ -replace '<[^>]+>', ' ' -replace '\s+', ' ' } |
                Where-Object { $_.Trim() } | Select-Object -Last 40 | Write-Host
        }
}

# --- 5. Render + run one program ---------------------------------------------
# Returns $true when the program's html exists at $outHtml afterwards.
function Invoke-AnalysisProgram {
    param(
        [string]$PgmPath,
        [string]$OutHtml,
        [hashtable]$Substitutions,
        [string]$Pgm7
    )
    $pgmText = Get-Content $PgmPath -Raw
    foreach ($key in $Substitutions.Keys) {
        $pgmText = $pgmText.Replace($key, ($Substitutions[$key] -replace '\\', '/'))
    }
    Set-Content -Path $Pgm7 -Value $pgmText -Encoding ASCII
    Write-Host "run: rendered $Pgm7"
    Get-Content $Pgm7 | Write-Host

    $proc = Start-Process -FilePath $analysis -ArgumentList "`"$Pgm7`"" `
        -WorkingDirectory $exeDir -PassThru
    Write-Host "run: started PID $($proc.Id), waiting up to ${TimeoutSeconds}s"
    if (-not $proc.WaitForExit($TimeoutSeconds * 1000)) {
        $proc.Refresh()
        Write-Host "run: timeout — MainWindowTitle='$($proc.MainWindowTitle)' Responding=$($proc.Responding)"
        Write-WindowDiagnostics -ProcessId $proc.Id
        $proc.Kill()
        Write-Error "run: Analysis.exe did not exit within ${TimeoutSeconds}s for $Pgm7"
        return $false
    }
    Write-Host "run: Analysis.exe exited with code $($proc.ExitCode)"
    $deadline = (Get-Date).AddSeconds(15)
    while (-not (Test-Path $OutHtml) -and (Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
    }
    if (-not (Test-Path $OutHtml)) {
        Write-Error "run: $OutHtml was not produced"
        return $false
    }
    Write-Host "run: OK — $OutHtml ($((Get-Item $OutHtml).Length) bytes)"
    return $true
}

$failures = @()

# --- 6. oswego.pgm ------------------------------------------------------------
$outHtml = Join-Path $outDir "oswego.html"
$ok = Invoke-AnalysisProgram -PgmPath (Join-Path $PSScriptRoot "oswego.pgm") `
    -OutHtml $outHtml -Pgm7 (Join-Path $workDir "oswego.pgm7") `
    -Substitutions @{ '@SAMPLE_MDB@' = $mdb; '@OUTPUT_HTML@' = $outHtml }
if (-not $ok) { $failures += "oswego" }

# --- 7. v11/*.pgm corpus ------------------------------------------------------
Get-ChildItem (Join-Path $PSScriptRoot "v11") -Filter "*.pgm" | Sort-Object Name | ForEach-Object {
    $name = $_.BaseName
    $vHtml = Join-Path $v11Out "$name.html"
    $ok = Invoke-AnalysisProgram -PgmPath $_.FullName -OutHtml $vHtml `
        -Pgm7 (Join-Path $workDir "$name.pgm7") `
        -Substitutions @{ '@WORK_DIR@' = $v11Data; '@OUTPUT_HTML@' = $vHtml }
    if (-not $ok) { $failures += $name }
}

if ($failures.Count -gt 0) {
    Write-Error "run: failed program(s): $($failures -join ', ')"
    exit 1
}
Write-Host "run: all programs produced output"
exit 0
