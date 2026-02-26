# MiniMax M2.5 — Tool Calling Skill

This skill configures ZeroClaw's agent loop to work reliably with MiniMax-M2.5,
avoiding the most common failures: rejected `system` role, malformed tool call
arguments, and silent tool dispatch errors.

## System Prompt

Use the following as your agent system prompt when running with MiniMax-M2.5:

```
You are ZeroClaw, a fast and reliable AI assistant.

## Tool Usage Rules
- When the user asks you to do something you have a tool for, ALWAYS call the tool.
- Never fabricate tool results. If a tool call fails, report the error clearly.
- Always use the EXACT parameter names defined in the tool schema. Do not rename fields.
- Tool arguments MUST be valid JSON. Never add trailing commas or comments inside JSON.
- If a task requires multiple steps, execute them one tool at a time and wait for results.
- After receiving a tool result, always incorporate it into your response before calling the next tool.

## Response Rules
- Be concise. Avoid unnecessary preamble.
- When you do not have a tool for something, say so clearly instead of guessing.
- Respond in the same language the user writes in.
```

## Config Snippet (`~/.zeroclaw/config.toml`)

```toml
# Provider — use native minimax, NOT anthropic-custom, for tool calling
api_key          = "YOUR_MINIMAX_API_KEY"
default_provider = "minimax"
default_model    = "MiniMax-M2.5"
default_temperature = 0.7

[agent]
compact_context       = false
max_tool_iterations   = 10
max_history_messages  = 50
parallel_tools        = false   # keep false for MiniMax — parallel dispatch causes ID collisions
tool_dispatcher       = "auto"
```

## Why `parallel_tools = false`

MiniMax-M2.5 assigns sequential tool call IDs. If ZeroClaw dispatches tools in
parallel, the response order can mismatch the ID sequence and the provider rejects
the conversation with an "invalid tool ID" error. Setting `parallel_tools = false`
enforces sequential dispatch and eliminates this class of error.

## Common Errors and Fixes

| Error | Cause | Fix |
|-------|-------|-----|
| `invalid tool ID` | Parallel tool dispatch with mismatched IDs | Set `parallel_tools = false` |
| `system role not supported` | Using `anthropic-custom` endpoint | Switch to native `minimax` provider |
| Tool called but no result | `max_tool_iterations` too low | Increase to `10` or higher |
| Model returns raw JSON instead of calling tool | Temperature too high | Set `default_temperature = 0.7` or lower |
| Empty tool arguments `{}` | Tool description too vague | Add concrete parameter descriptions in tool schema |

## Telegram-Specific Notes

When using MiniMax-M2.5 via the Telegram channel:

- Keep messages short — long context windows increase the risk of tool call
  schema truncation.
- If the bot stops responding mid-conversation, it is likely stuck waiting for
  a tool result. Send `/reset` or `/clear` to start a fresh context.
- Stream mode should remain `"off"` for MiniMax (stream parsing differs from
  OpenAI format and can drop tool call deltas).

```toml
[channels_config.telegram]
bot_token            = "YOUR_BOT_TOKEN"
stream_mode          = "off"
draft_update_interval_ms = 1000
```

## Related Docs

- [MiniMax Setup Guide](../docs/minimax-setup.md)
- [Providers Reference](../docs/providers-reference.md)
- [Custom Providers](../docs/custom-providers.md)
