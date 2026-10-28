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

# --- 2. Locate the Oswego sample database ------------------------------------
$sampleMdb = Join-Path $exeDir "Projects\Sample\Sample.mdb"
if (-not (Test-Path $sampleMdb)) {
    $sampleMdb = Get-ChildItem -Path $ei7 -Recurse -Filter "Sample.mdb" -File -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match 'Sample' } |
        Select-Object -First 1 -ExpandProperty FullName
}
if (-not $sampleMdb -or -not (Test-Path $sampleMdb)) {
    Write-Error "run-pgm: Sample.mdb (Oswego sample database) not found"
    exit 1
}
Write-Host "run-pgm: Sample.mdb=$sampleMdb"

# --- 3. Render the .pgm7 -------------------------------------------------------
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$outHtml = Join-Path $outDir "oswego-tables.html"
$pgmText = Get-Content (Join-Path $PSScriptRoot "oswego-tables.pgm") -Raw
$pgmText = $pgmText.Replace("@SAMPLE_MDB@", ($sampleMdb -replace '\\', '/'))
$pgmText = $pgmText.Replace("@OUTPUT_HTML@", ($outHtml -replace '\\', '/'))

$workDir = Join-Path $outDir "work"
New-Item -ItemType Directory -Force -Path $workDir | Out-Null
$pgm7 = Join-Path $workDir "oswego-tables.pgm7"
Set-Content -Path $pgm7 -Value $pgmText -Encoding ASCII
Write-Host "run-pgm: rendered $pgm7"
Get-Content $pgm7 | Write-Host

# --- 4. Run Analysis.exe and wait for QUIT ------------------------------------
function Write-WindowDiagnostics {
    param([int]$ProcessId, [string]$ExeDir)
    # Dump the UI Automation tree of the hung process: on a headless CI
    # runner the window station is invisible, so UIA is the only way to see
    # which dialog (if any) is blocking the message loop.
    try {
        Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
        $root = [System.Windows.Automation.AutomationElement]::RootElement
        $cond = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ProcessIdProperty, $ProcessId)
        $wins = $root.FindAll([System.Windows.Automation.TreeScope]::Children, $cond)
        Write-Host "run-pgm: $($wins.Count) top-level window(s) for PID $ProcessId"
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
    } catch { Write-Host "run-pgm: UIA dump failed: $_" }
    foreach ($sub in "Configuration", "Output", "Logs") {
        $d = Join-Path $ExeDir $sub
        if (Test-Path $d) {
            Write-Host "run-pgm: $d"
            Get-ChildItem $d -Recurse -File | ForEach-Object {
                Write-Host ("    {0} ({1} bytes, {2})" -f $_.FullName, $_.Length, $_.LastWriteTime)
            }
        }
    }
    # Epi Info writes each run's output to <config output dir>\output*.html —
    # its text shows which command failed and why.
    Get-ChildItem -Path $ExeDir -Recurse -Filter "output*.html" -File -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 2 | ForEach-Object {
            Write-Host "run-pgm: --- $($_.FullName) ---"
            Get-Content $_.FullName | ForEach-Object { $_ -replace '<[^>]+>', ' ' -replace '\s+', ' ' } |
                Where-Object { $_.Trim() } | Select-Object -Last 40 | Write-Host
        }
    Get-ChildItem -Path $ExeDir -Recurse -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -in @('IHistory.html','IResults.html') -or $_.Extension -eq '.txt' } |
        ForEach-Object {
            Write-Host "run-pgm: --- $($_.FullName) ---"
            Get-Content $_.FullName | ForEach-Object { $_ -replace '<[^>]+>', ' ' -replace '\s+', ' ' } |
                Where-Object { $_.Trim() } | Select-Object -Last 20 | Write-Host
        }
}

$proc = Start-Process -FilePath $analysis -ArgumentList "`"$pgm7`"" `
    -WorkingDirectory $exeDir -PassThru
Write-Host "run-pgm: started PID $($proc.Id), waiting up to ${TimeoutSeconds}s"

if (-not $proc.WaitForExit($TimeoutSeconds * 1000)) {
    $proc.Refresh()
    Write-Host "run-pgm: timeout — MainWindowTitle='$($proc.MainWindowTitle)' Responding=$($proc.Responding)"
    Write-WindowDiagnostics -ProcessId $proc.Id -ExeDir $exeDir
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
