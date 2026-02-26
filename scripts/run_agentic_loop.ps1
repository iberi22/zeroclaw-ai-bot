param(
    [string]$RepoRoot = "E:\scripts-python\zeroclaw",
    [string]$ExePath = "C:\Users\belal\.cargo\target_global\release\zeroclaw.exe",
    [string]$ProfileRoot = "C:\Users\belal\.zeroclaw-openclaw-bridge",
    [string]$TasksFile = "benchmarks\agent_tasks.json",
    [int]$Loops = 3,
    [double]$MinPassRate = 70.0,
    [double]$MinScoreRatio = 0.70,
    [switch]$NotifyOnPass
)

$ErrorActionPreference = "Stop"

Set-Location $RepoRoot

# 1) Keep bridge profile in sync with OpenClaw (read-only source)
python scripts\openclaw_workspace_clone.py --target-root $ProfileRoot | Out-Host

# 2) Run multi-loop benchmark with heuristic tuning and self-analysis
python scripts\agent_benchmark.py `
  --exe $ExePath `
  --tasks $TasksFile `
  --profile-root $ProfileRoot `
  --agentic-loops $Loops `
  --apply-heuristics `
  --self-analyze | Out-Host

# 3) Find latest summary
$runsDir = Join-Path $ProfileRoot "benchmarks\runs"
$latestSummary = Get-ChildItem $runsDir -Filter "*.summary.json" |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 1

if (-not $latestSummary) {
    throw "No summary file found in $runsDir"
}

Write-Host "Latest summary: $($latestSummary.FullName)"

# 4) Apply quality gate
python scripts\benchmark_gate.py `
  --summary $latestSummary.FullName `
  --min-pass-rate $MinPassRate `
  --min-score-ratio $MinScoreRatio | Out-Host

$gateCode = $LASTEXITCODE

# 5) Send notifications (on failure by default; pass when -NotifyOnPass)
$notifyArgs = @(
    "-NoProfile",
    "-ExecutionPolicy", "Bypass",
    "-File", "scripts\notify_agentic_result.ps1",
    "-SummaryPath", $latestSummary.FullName,
    "-GateCode", "$gateCode",
    "-MinPassRate", "$MinPassRate",
    "-MinScoreRatio", "$MinScoreRatio"
)
if ($NotifyOnPass) {
    $notifyArgs += "-NotifyOnPass"
}
powershell @notifyArgs | Out-Host

# 6) Append lightweight historical leaderboard row
$leaderboard = Join-Path $runsDir "leaderboard.csv"
if (-not (Test-Path $leaderboard)) {
    "timestamp,summary,gate_exit_code,min_pass_rate,min_score_ratio" | Out-File $leaderboard -Encoding utf8
}
$ts = Get-Date -Format "yyyy-MM-ddTHH:mm:ssK"
"$ts,$($latestSummary.Name),$gateCode,$MinPassRate,$MinScoreRatio" | Out-File $leaderboard -Encoding utf8 -Append

exit $gateCode
