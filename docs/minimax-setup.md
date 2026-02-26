# MiniMax Setup

ZeroClaw supports MiniMax natively via the `minimax` provider family, and also through
the Anthropic-compatible endpoint at `https://api.minimax.io/anthropic`.

## Overview

| Method | Provider string | Endpoint |
|--------|----------------|----------|
| Native (recommended) | `minimax` | `https://api.minimax.io/v1` |
| Anthropic-compatible | `anthropic-custom:https://api.minimax.io/anthropic` | `https://api.minimax.io/anthropic` |
| China region | `minimax-cn` | `https://api.minimax.chat/v1` |

Both methods support MiniMax-M2.5. The **native provider is preferred** because it
handles tool calls correctly without schema translation.

## Quick Start

### Option A — Native Provider (Recommended for tool calling)

```bash
zeroclaw onboard --provider minimax --api-key "YOUR_MINIMAX_API_KEY"
```

### Option B — Anthropic-Compatible Endpoint

```bash
zeroclaw onboard `
  --provider "anthropic-custom:https://api.minimax.io/anthropic" `
  --api-key "YOUR_MINIMAX_API_KEY"
```

> **Note**: On Windows PowerShell, use `` ` `` for line continuation instead of `\`.

## Manual Configuration

Edit `~/.zeroclaw/config.toml`:

### Native provider (preferred)

```toml
api_key = "YOUR_MINIMAX_API_KEY"
default_provider = "minimax"
default_model = "MiniMax-M2.5"
default_temperature = 0.7
```

### Anthropic-compatible endpoint

```toml
api_key = "YOUR_MINIMAX_API_KEY"
default_provider = "anthropic-custom:https://api.minimax.io/anthropic"
default_model = "MiniMax-M2.5"
default_temperature = 0.7
```

## Environment Variables

ZeroClaw resolves credentials in this order:

1. `api_key` in `config.toml`
2. `MINIMAX_API_KEY`
3. `MINIMAX_OAUTH_TOKEN`
4. Generic fallback: `ZEROCLAW_API_KEY` or `API_KEY`

Add to your `.env` file or set in your shell:

```bash
MINIMAX_API_KEY=your-api-key-here
```

## Available Models

| Model | Notes |
|-------|-------|
| `MiniMax-M2.5` | Default; best reasoning and tool calling |
| `MiniMax-Text-01` | Lightweight, fast responses |

## Tool Calling with MiniMax-M2.5

MiniMax-M2.5 supports function/tool calling. To avoid failures, apply the
optimized skill from `SKILLS/minimax-tool-calling.md`.

Key requirements for reliable tool calls:

- Use the **native `minimax` provider**, not `anthropic-custom`, to avoid
  Anthropic-to-OpenAI schema translation mismatches.
- Enable reasoning split for MiniMax-M2.5 tool loops (`extra_body.reasoning_split=true`)
  when using OpenAI-compatible clients.
- Keep tool descriptions short and unambiguous (< 80 chars per field description).
- Provide a `system` prompt that explicitly instructs the model to call tools
  when relevant, and to return valid JSON arguments.
- Preserve assistant tool-call payloads and return tool results with
  `role="tool"` + exact `tool_call_id` values.

## Verify Setup

### Test via curl (native endpoint)

```powershell
$headers = @{
    "Authorization" = "Bearer $env:MINIMAX_API_KEY"
    "Content-Type"  = "application/json"
}
$body = @{
    model    = "MiniMax-M2.5"
    messages = @(@{ role = "user"; content = "Hello" })
} | ConvertTo-Json -Depth 5

Invoke-RestMethod -Uri "https://api.minimax.io/v1/chat/completions" `
    -Method POST -Headers $headers -Body $body
```

### Test via ZeroClaw CLI

```bash
zeroclaw agent -m "Hello, who are you?"
zeroclaw status
```

## Telegram Bot Setup

When using MiniMax-M2.5 in the Telegram channel, ensure:

1. `config.toml` has the correct `bot_token` in `[channels_config.telegram]`
2. `default_provider` is set to `minimax` (native) for proper tool dispatch
3. Apply the `minimax-tool-calling` skill as `[agent]` config — see
   `SKILLS/minimax-tool-calling.md`

## Troubleshooting

### Tool calls returning malformed JSON

**Symptom:** `invalid tool ID` or tools not being invoked.

**Solution:**
- Switch from `anthropic-custom:...` to the native `minimax` provider.
- Enable the `minimax-tool-calling` skill.
- Reduce tool description complexity.

### 401 / 403 Authentication Errors

**Symptom:** API returns `Unauthorized`.

**Solution:**
- Verify the `api_key` is set in `config.toml` or via `MINIMAX_API_KEY`.
- Ensure no extra whitespace in the key value.
- Confirm the key has not expired on the MiniMax dashboard.

### `system` Role Rejection

**Symptom:** Error mentioning unsupported `system` role.

**Solution:**
- Use the `anthropic-custom` endpoint which handles system prompts in the
  Anthropic format, or configure a skill that injects the system message as
  the first `user` turn.

### Model Not Found

```powershell
# List available models
$headers = @{ "Authorization" = "Bearer $env:MINIMAX_API_KEY" }
Invoke-RestMethod -Uri "https://api.minimax.io/v1/models" -Headers $headers |
    Select-Object -ExpandProperty data | Select-Object id
```

## Getting an API Key

1. Go to [MiniMax Platform](https://www.minimaxi.com) (global) or
   [api.minimax.io](https://api.minimax.io)
2. Create an account and navigate to **API Keys**
3. Generate a new key and copy it

## Related Documentation

- [ZeroClaw Providers Reference](./providers-reference.md)
- [Custom Provider Endpoints](./custom-providers.md)
- [Channels Reference](./channels-reference.md)
- [MiniMax Tool Calling Skill](../SKILLS/minimax-tool-calling.md)
