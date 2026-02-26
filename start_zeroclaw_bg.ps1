$Binary = "C:\Users\belal\.cargo\target_global\release\zeroclaw.exe"
$OpenClawConfig = "C:\Users\belal\.clawdbot\openclaw.json"

if (-not (Test-Path $Binary)) {
    Write-Host "Binary not found at $Binary. Please run build first."
    exit 1
}

if (Test-Path $OpenClawConfig) {
    try {
        $cfg = Get-Content $OpenClawConfig -Raw | ConvertFrom-Json
        $openClawWorkspace = $cfg.agents.defaults.workspace
        if ($openClawWorkspace) {
            # ZEROCLAW_WORKSPACE expects the ZeroClaw config root, not the OpenClaw workspace
            # itself. Use a dedicated bridge root so ZeroClaw keeps an isolated config/workspace
            # while still being able to read OpenClaw resources via integration sync.
            $bridgeRoot = Join-Path $Env:USERPROFILE ".zeroclaw-openclaw-bridge"
            New-Item -ItemType Directory -Force -Path $bridgeRoot | Out-Null
            $Env:ZEROCLAW_WORKSPACE = $bridgeRoot
        }

        if (-not $Env:ANTHROPIC_API_KEY) {
            $minimax = $cfg.models.providers.minimax
            if ($minimax.apiKey) {
                $Env:ANTHROPIC_API_KEY = $minimax.apiKey
            }
        }
    }
    catch {
        Write-Host "Warning: could not parse $OpenClawConfig, using existing environment values."
    }
}

# Start in background
Start-Process -FilePath $Binary -ArgumentList "daemon" -WindowStyle Hidden -WorkingDirectory "E:\scripts-python\zeroclaw"
