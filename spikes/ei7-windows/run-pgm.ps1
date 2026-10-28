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
$sampleMdb = Join-Path $exeDir "Projects\Sample\Sample.mdb"
if (-not (Test-Path $samplePrj)) {
    $samplePrj = Get-ChildItem -Path $ei7 -Recurse -Filter "Sample.prj" -File -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match 'Sample' } |
        Select-Object -First 1 -ExpandProperty FullName
}
if (-not (Test-Path $sampleMdb)) {
    $sampleMdb = Get-ChildItem -Path $ei7 -Recurse -Filter "Sample.mdb" -File -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match 'Sample' } |
        Select-Object -First 1 -ExpandProperty FullName
}
if (-not $samplePrj -or -not $sampleMdb) {
    Write-Error "run-pgm: Sample.prj / Sample.mdb (Oswego sample project) not found"
    exit 1
}
Write-Host "run-pgm: Sample.prj=$samplePrj"
Write-Host "run-pgm: Sample.mdb=$sampleMdb"

# --- 3. Seed and patch EpiInfo.Config.xml -------------------------------------
# The shipped .prj files' connection strings are FipsCrypto-encrypted; the
# default generated config leaves TextEncryptionModule/FileName empty, which
# makes Epi.Configuration.Decrypt use the built-in key and fail. An installed
# copy names the module, so do the same: ensure the config exists (launching
# Analysis.exe creates it), then point it at FipsCrypto.dll next to the exe.
$configDir  = Join-Path $exeDir "Configuration"
$configFile = Join-Path $configDir "EpiInfo.Config.xml"
if (-not (Test-Path $configFile)) {
    Write-Host "run-pgm: seeding $configFile via a brief Analysis.exe launch"
    $seed = Start-Process -FilePath $analysis -WorkingDirectory $exeDir -PassThru
    $deadline = (Get-Date).AddSeconds(60)
    $stable = 0
    while ((Get-Date) -lt $deadline -and $stable -lt 3) {
        Start-Sleep -Seconds 2
        if (Test-Path $configFile) { $stable++ }
    }
    if (-not $seed.HasExited) { $seed.Kill() }
}
if (Test-Path $configFile) {
    $cfg = [xml](Get-Content $configFile -Raw)
    $tem = $cfg.SelectSingleNode("//*[local-name()='TextEncryptionModule']")
    $fips = Join-Path $exeDir "FipsCrypto.dll"
    Write-Host "run-pgm: FipsCrypto.dll present: $(Test-Path $fips)"
    if ($tem -and (Test-Path $fips)) {
        $fn = $tem.SelectSingleNode("*[local-name()='FileName']")
        if (-not $fn) { $fn = $cfg.CreateElement("FileName"); [void]$tem.AppendChild($fn) }
        Write-Host "run-pgm: TextEncryptionModule/FileName was '$($fn.InnerText)' -> '$fips'"
        $fn.InnerText = $fips
        $cfg.Save($configFile)
    } else {
        Write-Host "run-pgm: no TextEncryptionModule node or no FipsCrypto.dll — .prj reads will fail"
    }
} else {
    Write-Host "run-pgm: $configFile was not created — .prj reads will fail"
}

# --- 4. Render the .pgm7 -------------------------------------------------------
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$workDir = Join-Path $outDir "work"
New-Item -ItemType Directory -Force -Path $workDir | Out-Null
$outHtml = Join-Path $outDir "oswego-tables.html"
$pgmBase = Get-Content (Join-Path $PSScriptRoot "oswego-tables.pgm") -Raw

function New-Pgm7([string]$text, [string]$prj, [string]$html, [string]$name) {
    $t = $text.Replace("@SAMPLE_PRJ@", ($prj -replace '\\', '/'))
    $t = $t.Replace("@OUTPUT_HTML@", ($html -replace '\\', '/'))
    $p = Join-Path $workDir $name
    Set-Content -Path $p -Value $t -Encoding ASCII
    Write-Host "run-pgm: rendered $p"
    Get-Content $p | Write-Host
    return $p
}

$pgm7 = New-Pgm7 $pgmBase $samplePrj $outHtml "oswego-tables.pgm7"

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

# --- 5. Run Analysis.exe and wait for QUIT ------------------------------------
function Invoke-Analysis {
    param([string]$Pgm7, [string]$ExpectedHtml)
    $proc = Start-Process -FilePath $analysis -ArgumentList "`"$Pgm7`"" `
        -WorkingDirectory $exeDir -PassThru
    Write-Host "run-pgm: started PID $($proc.Id), waiting up to ${TimeoutSeconds}s"
    if (-not $proc.WaitForExit($TimeoutSeconds * 1000)) {
        $proc.Refresh()
        Write-Host "run-pgm: timeout — MainWindowTitle='$($proc.MainWindowTitle)' Responding=$($proc.Responding)"
        Write-WindowDiagnostics -ProcessId $proc.Id -ExeDir $exeDir
        $proc.Kill()
        return $false
    }
    Write-Host "run-pgm: Analysis.exe exited with code $($proc.ExitCode)"
    $deadline = (Get-Date).AddSeconds(15)
    while (-not (Test-Path $ExpectedHtml) -and (Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
    }
    return (Test-Path $ExpectedHtml)
}

$ok = Invoke-Analysis -Pgm7 $pgm7 -ExpectedHtml $outHtml
if (-not $ok) {
    # Fallback diagnostic attempt: read Sample.mdb directly (no .prj crypto)
    # and LIST all fields — the listing reveals the physical column names so
    # the TABLES line can be fixed. Derived from the committed .pgm by string
    # replacement only; recorded in the spike report.
    Write-Host "run-pgm: primary attempt produced no output; trying mdb LIST fallback"
    $fbHtml = Join-Path $outDir "oswego-tables-list.html"
    $fbText = $pgmBase `
        -replace [regex]::Escape("READ {@SAMPLE_PRJ@}:Oswego"), "READ '@SAMPLE_MDB@':Oswego" `
        -replace "(?m)^TABLES VANILLA ILL.*$", "LIST *"
    $fbText = $fbText.Replace("@SAMPLE_MDB@", ($sampleMdb -replace '\\', '/'))
    $fb7 = New-Pgm7 $fbText $samplePrj $fbHtml "oswego-tables-list.pgm7"
    $ok = Invoke-Analysis -Pgm7 $fb7 -ExpectedHtml $fbHtml
    if (-not $ok) {
        Write-Error "run-pgm: neither the .prj READ nor the .mdb LIST produced output — headless run failed"
        exit 1
    }
    Write-Host "run-pgm: fallback produced $fbHtml (LIST output; TABLES VANILLA ILL still unproven)"
    exit 1
}
Write-Host "run-pgm: OK — $outHtml ($((Get-Item $outHtml).Length) bytes)"
exit 0
