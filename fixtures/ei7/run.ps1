# run.ps1 — run fixtures/ei7/oswego.pgm through Epi Info 7 Classic Analysis.
#
# Epiora v0.2 Task 3. Promoted from spikes/ei7-windows/run-pgm.ps1 and
# narrowed to the single route the ei7.yml workflow uses: the official
# 7.2.7.0 Analysis.exe and the Sample.mdb inside the release ZIP (the
# .prj-view and source-built routes of the spike are not repeated here —
# see docs/spikes/ei7-windows.md for why .prj READs cannot work on CI).
#
# Locates Analysis.exe under -Ei7Dir (the extracted release ZIP) or honours
# EI7_ANALYSIS_EXE, renders oswego.pgm into a *.pgm7 file with absolute
# paths — Analysis.exe's command-line scan only accepts arguments whose
# file name contains ".pgm7" — then runs it. The program's own QUIT
# statement closes the application, so a timeout is a headless-run
# failure, not normal.
#
# Paths substituted into the program use forward slashes: inside .pgm
# BraceString and String literals a backslash escapes the next character.
#
# Exit code 0 only if -OutputDir contains oswego.html afterwards.

[CmdletBinding()]
param(
    # Root searched recursively for Analysis.exe (extracted release ZIP).
    [string]$Ei7Dir = "ei7-rel",
    # Directory that receives oswego.html; asserted on by the workflow.
    [string]$OutputDir = "ei7-out",
    # Headless run guard: Analysis.exe is killed after this many seconds.
    [int]$TimeoutSeconds = 300
)

$ErrorActionPreference = "Stop"
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

# --- 3. Render the .pgm7 ----------------------------------------------------
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$workDir = Join-Path $outDir "work"
New-Item -ItemType Directory -Force -Path $workDir | Out-Null
$outHtml = Join-Path $outDir "oswego.html"
$pgmText = Get-Content (Join-Path $PSScriptRoot "oswego.pgm") -Raw
$pgmText = $pgmText.Replace("@SAMPLE_MDB@", ($mdb -replace '\\', '/'))
$pgmText = $pgmText.Replace("@OUTPUT_HTML@", ($outHtml -replace '\\', '/'))
$pgm7 = Join-Path $workDir "oswego.pgm7"
Set-Content -Path $pgm7 -Value $pgmText -Encoding ASCII
Write-Host "run: rendered $pgm7"
Get-Content $pgm7 | Write-Host

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

# --- 5. Run Analysis.exe and wait for QUIT ----------------------------------
$proc = Start-Process -FilePath $analysis -ArgumentList "`"$pgm7`"" `
    -WorkingDirectory $exeDir -PassThru
Write-Host "run: started PID $($proc.Id), waiting up to ${TimeoutSeconds}s"
if (-not $proc.WaitForExit($TimeoutSeconds * 1000)) {
    $proc.Refresh()
    Write-Host "run: timeout — MainWindowTitle='$($proc.MainWindowTitle)' Responding=$($proc.Responding)"
    Write-WindowDiagnostics -ProcessId $proc.Id
    $proc.Kill()
    Write-Error "run: Analysis.exe did not exit within ${TimeoutSeconds}s"
    exit 1
}
Write-Host "run: Analysis.exe exited with code $($proc.ExitCode)"
$deadline = (Get-Date).AddSeconds(15)
while (-not (Test-Path $outHtml) -and (Get-Date) -lt $deadline) {
    Start-Sleep -Milliseconds 500
}
if (-not (Test-Path $outHtml)) {
    Write-Error "run: $outHtml was not produced"
    exit 1
}
Write-Host "run: OK — $outHtml ($((Get-Item $outHtml).Length) bytes)"
exit 0
