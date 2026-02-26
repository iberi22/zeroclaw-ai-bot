param(
    [string]$SummaryPath,
    [int]$GateCode,
    [double]$MinPassRate = 70.0,
    [double]$MinScoreRatio = 0.70,
    [switch]$NotifyOnPass
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $SummaryPath)) {
    throw "Summary file not found: $SummaryPath"
}

$summary = Get-Content $SummaryPath -Raw | ConvertFrom-Json
$last = $summary.loops[-1]
$runId = $summary.run_id
$loopIdx = $last.loop_index
$score = [double]$last.score
$maxScore = [double]$last.max_score
$passRate = [double]$last.pass_rate
$ratio = if ($maxScore -gt 0) { $score / $maxScore } else { 0.0 }
$status = if ($GateCode -eq 0) { "PASSED" } else { "FAILED" }

if ($GateCode -eq 0 -and -not $NotifyOnPass) {
    Write-Host "Notifications skipped (gate passed and NotifyOnPass=false)."
    exit 0
}

$message = @"
[ZeroClaw Agentic Loop] $status
run_id: $runId
loop: $loopIdx
score: $score/$maxScore (ratio=$([Math]::Round($ratio,3)))
pass_rate: $([Math]::Round($passRate,2))%
thresholds: pass_rate>=$MinPassRate score_ratio>=$MinScoreRatio
summary: $SummaryPath
"@

function Send-Discord([string]$webhookUrl, [string]$text) {
    if ([string]::IsNullOrWhiteSpace($webhookUrl)) { return $false }
    try {
        $body = @{ content = $text } | ConvertTo-Json -Compress
        Invoke-RestMethod -Uri $webhookUrl -Method Post -Body $body -ContentType "application/json" | Out-Null
        return $true
    } catch {
        Write-Host "Discord notification failed: $($_.Exception.Message)"
        return $false
    }
}

function Send-Telegram([string]$botToken, [string]$chatId, [string]$text) {
    if ([string]::IsNullOrWhiteSpace($botToken) -or [string]::IsNullOrWhiteSpace($chatId)) { return $false }
    try {
        $url = "https://api.telegram.org/bot$botToken/sendMessage"
        $payload = @{
            chat_id = $chatId
            text = $text
        }
        Invoke-RestMethod -Uri $url -Method Post -Body $payload | Out-Null
        return $true
    } catch {
        Write-Host "Telegram notification failed: $($_.Exception.Message)"
        return $false
    }
}

$discordWebhook = $env:ZEROCLAW_NOTIFY_DISCORD_WEBHOOK_URL
$telegramToken = $env:ZEROCLAW_NOTIFY_TELEGRAM_BOT_TOKEN
$telegramChat = $env:ZEROCLAW_NOTIFY_TELEGRAM_CHAT_ID

$discordSent = Send-Discord $discordWebhook $message
$telegramSent = Send-Telegram $telegramToken $telegramChat $message

if (-not $discordSent -and -not $telegramSent) {
    Write-Host "No notification channel configured or all notifications failed."
    exit 0
}

Write-Host "Notification sent. Discord=$discordSent Telegram=$telegramSent"
