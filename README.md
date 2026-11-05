# waybar-claude

A minimal [waybar](https://github.com/Alexays/Waybar) module showing how far you are into your Claude Code usage limits.

- A single icon in the bar, coloured by the busiest limit: yellow, orange from 50%, red from 80%, pulsing from 90%.
- Hover to slide out the 5-hour usage and time until reset.
- The tooltip lists the 5-hour and 7-day windows with bars and reset countdowns.

## How it works

Claude Code passes `rate_limits` to its statusline command. `waybar_claude record` reads that JSON from stdin and keeps the latest values in `~/.cache/waybar_claude/usage.json`. The module reads that file, so numbers refresh while a Claude Code session is open. A window past its reset time is shown as 0%.

## Install

```bash
cargo build --release
ln -s "$PWD/waybar" ~/.config/waybar/claude
```

Add to your statusline script, right after reading stdin:

```bash
input=$(cat)
"$HOME/.config/waybar/claude/run.sh" record <<<"$input" >/dev/null 2>&1 &
```

Waybar config:

```jsonc
"modules-left": ["group/group-claude"],
"include": ["~/.config/waybar/claude/modules.json"]
```

```css
@import "claude/style.css";
```

The icon is `format-icons` in `waybar/modules.json`. Colours use the `@primary`, `@inverse_primary` and `@error` waybar colour variables.

## Commands

```
waybar_claude [--cache FILE] [--watch] <show|detail|record>
```

Requires a Nerd Font and Rust (stable).
