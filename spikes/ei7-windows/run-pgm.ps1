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

# --- 1. Locate Analysis.exe (built and/or official install) -------------------
$built = Get-ChildItem -Path $ei7 -Recurse -Filter "Analysis.exe" -File -ErrorAction SilentlyContinue |
    Sort-Object { if ($_.FullName -match 'release') { 0 } else { 1 } }, FullName |
    Select-Object -First 1 -ExpandProperty FullName
$official = $env:EI7_ANALYSIS_EXE
if (-not $built -and -not ($official -and (Test-Path $official))) {
    Write-Error "run-pgm: no Analysis.exe found under $ei7 and EI7_ANALYSIS_EXE not set"
    exit 1
}
$analysis = if ($official -and (Test-Path $official)) { $official } else { $built }
Write-Host "run-pgm: built Analysis.exe=$built"
Write-Host "run-pgm: official Analysis.exe=$official"
Write-Host "run-pgm: primary Analysis.exe=$analysis"

# --- 2. Per-route Oswego sample resolution ------------------------------------
# Each route reads its own Sample.mdb so results are attributable: the
# official binary reads the copy shipped inside the release ZIP
# (<exeDir>\Projects\Sample\); the source-built binary reads the copy MSBuild
# placed next to it (<build output>\Projects\Sample\ — Epi.Core.csproj marks
# it CopyToOutputDirectory) or, failing that, the pinned checkout's
# Epi.Core\Projects\Sample\.
function Resolve-Sample([string]$ExeDir, [string]$CheckoutDir) {
    $prj = Join-Path $ExeDir "Projects\Sample\Sample.prj"
    if (-not (Test-Path $prj)) { $prj = $null }
    $mdb = Join-Path $ExeDir "Projects\Sample\Sample.mdb"
    if (-not (Test-Path $mdb)) { $mdb = $null }
    if ((-not $prj -or -not $mdb) -and $CheckoutDir) {
        $src = Join-Path $CheckoutDir "Epi.Core\Projects\Sample"
        $c = Join-Path $src "Sample.prj"
        if (-not $prj -and (Test-Path $c)) { $prj = $c }
        $c = Join-Path $src "Sample.mdb"
        if (-not $mdb -and (Test-Path $c)) { $mdb = $c }
    }
    return @{ Prj = $prj; Mdb = $mdb }
}

# --- 3. Seed and patch EpiInfo.Config.xml -------------------------------------
# The shipped .prj files' connection strings are FipsCrypto-encrypted; the
# default generated config leaves TextEncryptionModule/FileName empty, which
# makes Epi.Configuration.Decrypt use the built-in key and fail. An installed
# copy names the module, so do the same: ensure the config exists (launching
# Analysis.exe creates it), then point it at FipsCrypto.dll next to the exe.
# Runs per route so each binary's own Configuration\ is patched.
function Initialize-Ei7Config([string]$Exe, [string]$ExeDir) {
    $configDir  = Join-Path $ExeDir "Configuration"
    $configFile = Join-Path $configDir "EpiInfo.Config.xml"
    if (-not (Test-Path $configFile)) {
        Write-Host "run-pgm: seeding $configFile via a brief Analysis.exe launch"
        $seed = Start-Process -FilePath $Exe -WorkingDirectory $ExeDir -PassThru
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
        $fips = Join-Path $ExeDir "FipsCrypto.dll"
        Write-Host "run-pgm: FipsCrypto.dll present: $(Test-Path $fips)"
        if (Test-Path $fips) {
            if (-not $tem) {
                # An empty typed table serializes to no element; create the row so
                # config.TextEncryptionModule[0].FileName resolves.
                $tem = $cfg.CreateElement("TextEncryptionModule")
                foreach ($n in "DisplayName", "Type", "FileName") {
                    [void]$tem.AppendChild($cfg.CreateElement($n))
                }
                $tem.DisplayName = "FIPS cryptography"
                $tem.Type = "FipsCrypto"
                [void]$cfg.DocumentElement.AppendChild($tem)
                Write-Host "run-pgm: created TextEncryptionModule row (config had none)"
            }
            $fn = $tem.SelectSingleNode("*[local-name()='FileName']")
            if (-not $fn) { $fn = $cfg.CreateElement("FileName"); [void]$tem.AppendChild($fn) }
            Write-Host "run-pgm: TextEncryptionModule/FileName was '$($fn.InnerText)' -> '$fips'"
            $fn.InnerText = $fips
            $cfg.Save($configFile)
        } else {
            Write-Host "run-pgm: no FipsCrypto.dll — .prj reads will fail"
        }
    } else {
        Write-Host "run-pgm: $configFile was not created — .prj reads will fail"
    }
}

# --- 4. Render the .pgm7 -------------------------------------------------------
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$workDir = Join-Path $outDir "work"
New-Item -ItemType Directory -Force -Path $workDir | Out-Null
$outHtml = Join-Path $outDir "oswego-tables.html"
$pgmBase = Get-Content (Join-Path $PSScriptRoot "oswego-tables.pgm") -Raw

function New-Pgm7([string]$text, [string]$prj, [string]$mdb, [string]$html, [string]$name) {
    $t = $text.Replace("@SAMPLE_PRJ@", ($prj -replace '\\', '/'))
    $t = $t.Replace("@SAMPLE_MDB@", ($mdb -replace '\\', '/'))
    $t = $t.Replace("@OUTPUT_HTML@", ($html -replace '\\', '/'))
    $p = Join-Path $workDir $name
    Set-Content -Path $p -Value $t -Encoding ASCII
    Write-Host "run-pgm: rendered $p"
    Get-Content $p | Write-Host
    return $p
}

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
    param([string]$Exe, [string]$ExeDir, [string]$Pgm7, [string]$ExpectedHtml)
    $proc = Start-Process -FilePath $Exe -ArgumentList "`"$Pgm7`"" `
        -WorkingDirectory $ExeDir -PassThru
    Write-Host "run-pgm: started PID $($proc.Id) [$Exe], waiting up to ${TimeoutSeconds}s"
    if (-not $proc.WaitForExit($TimeoutSeconds * 1000)) {
        $proc.Refresh()
        Write-Host "run-pgm: timeout — MainWindowTitle='$($proc.MainWindowTitle)' Responding=$($proc.Responding)"
        Write-WindowDiagnostics -ProcessId $proc.Id -ExeDir $ExeDir
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

# Attempt matrix (each route gets its own ROUTEOUT target so every outcome
# is kept):
#   official + mdb Oswego12 TABLES -> oswego-tables.html       (release-ZIP sample)
#   built    + mdb Oswego12 TABLES -> oswego-tables-built.html (source-checkout
#                                                            sample; check target)
#   official + prj Oswego   TABLES -> oswego-tables-prj.html   (does the installed
#                                                            binary decrypt .prj?)
$attempts = @()
if ($official -and (Test-Path $official)) {
    $offDir = Split-Path $official -Parent
    $off = Resolve-Sample $offDir $null
    Write-Host "run-pgm: official route Sample.prj=$($off.Prj) Sample.mdb=$($off.Mdb)"
    if ($off.Mdb) {
        $attempts += @{ Label = "official+mdb"; Exe = $official; Dir = $offDir; Prj = $off.Prj; Mdb = $off.Mdb;
            Pgm = (New-Pgm7 $pgmBase $off.Prj $off.Mdb $outHtml "oswego-tables.pgm7"); Html = $outHtml }
        if ($off.Prj) {
            $prjHtml = Join-Path $outDir "oswego-tables-prj.html"
            $prjText = $pgmBase -replace [regex]::Escape("READ '@SAMPLE_MDB@':Oswego12"), "READ {@SAMPLE_PRJ@}:Oswego"
            $attempts += @{ Label = "official+prj"; Exe = $official; Dir = $offDir; Prj = $off.Prj; Mdb = $off.Mdb;
                Pgm = (New-Pgm7 $prjText $off.Prj $off.Mdb $prjHtml "oswego-tables-prj.pgm7"); Html = $prjHtml }
        }
    } else {
        Write-Host "run-pgm: no Sample.mdb under $offDir\Projects\Sample — official routes skipped"
    }
}
if ($built) {
    $bDir = Split-Path $built -Parent
    $bld = Resolve-Sample $bDir $ei7
    Write-Host "run-pgm: built route Sample.prj=$($bld.Prj) Sample.mdb=$($bld.Mdb)"
    if (-not $bld.Mdb) {
        Write-Error "run-pgm: Sample.mdb for the built route not found (looked in $bDir\Projects\Sample and $ei7\Epi.Core\Projects\Sample)"
        exit 1
    }
    $builtHtml = Join-Path $outDir "oswego-tables-built.html"
    $attempts += @{ Label = "built+mdb"; Exe = $built; Dir = $bDir; Prj = $bld.Prj; Mdb = $bld.Mdb;
        Pgm = (New-Pgm7 $pgmBase $bld.Prj $bld.Mdb $builtHtml "oswego-tables-built.pgm7"); Html = $builtHtml }
}

$produced = @()
$inited = @{}
foreach ($a in $attempts) {
    if (-not $inited[$a.Dir]) {
        Initialize-Ei7Config -Exe $a.Exe -ExeDir $a.Dir
        $inited[$a.Dir] = $true
    }
    Write-Host "run-pgm: === attempt $($a.Label) ==="
    if (Invoke-Analysis -Exe $a.Exe -ExeDir $a.Dir -Pgm7 $a.Pgm -ExpectedHtml $a.Html) {
        $produced += $a.Label
    }
}
Write-Host "run-pgm: routes that produced html: $($produced -join ', ')"

if ($produced.Count -eq 0) {
    # Fallback diagnostic attempt: LIST Oswego12 — reveals the real column
    # names so the TABLES line can be fixed. Derived from the committed .pgm
    # by string replacement only; recorded in the spike report.
    Write-Host "run-pgm: no output yet; trying LIST fallback on Oswego12"
    $fb = $attempts[0]
    $fbHtml = Join-Path $outDir "oswego-tables-list.html"
    $fbText = $pgmBase -replace "(?m)^TABLES VANILLA ILL.*$", "LIST *"
    $fb7 = New-Pgm7 $fbText $fb.Prj $fb.Mdb $fbHtml "oswego-tables-list.pgm7"
    $fbOk = Invoke-Analysis -Exe $fb.Exe -ExeDir $fb.Dir -Pgm7 $fb7 -ExpectedHtml $fbHtml
    Write-Error "run-pgm: TABLES VANILLA ILL produced no html on any route (LIST fallback produced html: $fbOk)"
    exit 1
}
Write-Host "run-pgm: OK — html outputs in ${outDir}:"
Get-ChildItem $outDir -Filter *.html -File | ForEach-Object { Write-Host "  $($_.Name) ($($_.Length) bytes)" }
exit 0
