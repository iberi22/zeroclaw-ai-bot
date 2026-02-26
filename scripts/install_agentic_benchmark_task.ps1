param(
    [string]$TaskName = "ZeroClaw Agentic Benchmark Loop",
    [string]$RepoRoot = "E:\scripts-python\zeroclaw",
    [string]$ScriptPath = "E:\scripts-python\zeroclaw\scripts\run_agentic_loop.ps1",
    [int]$RepeatMinutes = 360
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $ScriptPath)) {
    throw "Script not found: $ScriptPath"
}

function Invoke-Schtasks {
    param([string[]]$TaskArgs)
    $escaped = $TaskArgs | ForEach-Object {
        if ($_ -match "\s") { '"' + $_.Replace('"', '\"') + '"' } else { $_ }
    }
    $cmdLine = "schtasks " + ($escaped -join " ")
    $previousErrorPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $output = & cmd.exe /c $cmdLine 2>&1
    $code = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorPreference
    return @{
        Code = $code
        Output = ($output | Out-String)
    }
}

# Build a scheduled task that runs every N minutes indefinitely.
$escaped = $ScriptPath.Replace('"', '""')
$action = "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$escaped`""

try {
    $null = Invoke-Schtasks -TaskArgs @("/Delete", "/TN", $TaskName, "/F")
} catch {
    # Ignore when task does not exist yet.
}

$high = Invoke-Schtasks -TaskArgs @(
    "/Create",
    "/TN", $TaskName,
    "/SC", "MINUTE",
    "/MO", "$RepeatMinutes",
    "/TR", $action,
    "/RL", "HIGHEST",
    "/F"
)

if ($high.Code -ne 0) {
    Write-Host "High-privilege task install failed, retrying with LIMITED run level..."
    $limited = Invoke-Schtasks -TaskArgs @(
        "/Create",
        "/TN", $TaskName,
        "/SC", "MINUTE",
        "/MO", "$RepeatMinutes",
        "/TR", $action,
        "/RL", "LIMITED",
        "/F"
    )
    if ($limited.Code -ne 0) {
        Write-Host ($high.Output | Out-String)
        Write-Host ($limited.Output | Out-String)
        throw "Failed to create scheduled task '$TaskName'."
    }
    Write-Host ($limited.Output | Out-String)
} else {
    Write-Host ($high.Output | Out-String)
}

Write-Host "Installed scheduled task: $TaskName"
Write-Host "Runs every $RepeatMinutes minutes."
Write-Host "Start now: schtasks /Run /TN `"$TaskName`""
