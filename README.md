# 📼 Tapes-v2

> **Atomic, zero-downtime, per-target dotfiles switcher for Linux.**  
> Swap complete desktop environments and dotfile profiles in microseconds using kernel-level atomic directory exchanges.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust Edition: 2024](https://img.shields.io/badge/Rust-2024-orange.svg)](Cargo.toml)
[![Release](https://img.shields.io/github/v/release/futurekismo-oss/tapes-v2?color=brightgreen)](https://github.com/futurekismo-oss/tapes-v2/releases)

---

## 💡 Why Tapes?

Most dotfile managers (like GNU Stow or manual symlink managers) operate file-by-file. If a script crashes midway or you hit an error, your `~/.config` is left in a broken, half-swapped state that breaks running applications.

**Tapes solves this using Linux kernel primitives:**

1. **Zero Half-States**: Tapes constructs your entire new config tree off to the side in a staging directory (`~/.config.tapes`).
2. **Atomic Kernel Swap**: It swaps your active `~/.config` and the staged directory in a single, instantaneous syscall (`renameat2` with `RENAME_EXCHANGE`).
3. **Safe, Instant Rollback**: Run `tape eject`, and your original configuration is atomically swapped right back. Your real files are never modified, overwritten, or deleted.

```
   Existing Config                 Staged Overlay                   After Atomic Swap
  ┌─────────────────┐             ┌─────────────────┐             ┌─────────────────┐
  │ ~/.config/      │             │ ~/.config.tapes │             │ ~/.config/      │
  │  ├── nvim/      │             │  ├── nvim/ ───┐ │  renameat2  │  ├── nvim/ (sym)│ ──► [Tape]
  │  ├── hypr/      │             │  ├── hypr/ ───┼─┼───────────►│  ├── hypr/ (sym)│ ──► [Tape]
  │  └── waybar/    │             │  └── waybar/ ─┘ │ (EXCHANGE)  │  └── waybar/    │ ──► [Original]
  └─────────────────┘             └─────────────────┘             └─────────────────┘
           │                                                               │
           └────────────────── Swapped to Backup Side ─────────────────────┘
```

---

## ⚡ Quickstart

Install a tape, insert it into your desktop, and eject when you want your old setup back:

```bash
# 1. Install a community tape (from GitHub or local folder)
tape install futurekismo-oss/neon-testing --name neon

# 2. Inspect the tape and see what it overrides
tape show neon

# 3. Insert the tape (atomically switches your config and runs reload hooks)
tape insert neon

# 4. Check active status
tape status

# 5. Revert back to your original setup like nothing happened
tape eject
```

---

## 📦 Installation

### Option 1: Automatic Script (Recommended for Arch & Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/futurekismo-oss/tapes-v2/main/install.sh | bash
```

### Option 2: Nix / NixOS

Run directly without installing:
```bash
nix run github:futurekismo-oss/tapes-v2 -- --help
```

Or add to your Nix profile:
```bash
nix profile install github:futurekismo-oss/tapes-v2
```

### Option 3: From Source (Cargo)

```bash
git clone https://github.com/futurekismo-oss/tapes-v2.git
cd tapes-v2
cargo install --path .
```

---

## 🛠️ Command Reference

### `tape install <SOURCE>`
Downloads or copies a tape into your local library (`~/.tapes/`).
- Accepts **GitHub shorthands** (`owner/repo`), **full Git URLs** (`https://...` or `git@...`), or **local directories** (`~/my-dots`, `./path`).
- Automatically generates an example `tape.toml` if the repository lacks one.

```bash
# From GitHub shorthand
tape install user/hypr-dots

# From full Git URL with a custom name
tape install https://github.com/user/dots.git --name catppuccin

# From a specific branch
tape install user/dots -b dev

# From a local directory
tape install ~/Downloads/cool-theme --name cool-theme
```

---

### `tape insert [NAME]`
Atomically swaps the specified tape into your active `~/.config`.
- Resolves all targets: folders/files provided by the tape override your current config; any configs the tape does not touch are preserved.
- Automatically executes `[hooks.insert]` (e.g. window manager/bar reloads).

```bash
# Insert a tape from your ~/.tapes library
tape insert neon

# Test with a dry run (shows the plan without touching the filesystem)
tape insert neon --dry-run

# Insert without running hook scripts
tape insert neon --no-hooks
```

---

### `tape eject`
Atomically restores your original `~/.config` and cleanly deletes the staging directory.
- Runs `[hooks.eject]` commands to reset running desktop components.

```bash
# Revert back to default configuration
tape eject

# Dry-run test
tape eject --dry-run
```

---

### `tape status`
Displays the currently active tape, its source path, and target link records.

```bash
tape status

# Detailed verbose view
tape status -v
```

---

### `tape show [NAME]`
Lists all installed tapes in `~/.tapes`, or inspects a specific tape's manifest, description, and hooks.

```bash
# List all tapes in ~/.tapes
tape show

# Inspect manifest details for a tape
tape show neon
```

---

## 🎨 Creating Your Own Tape

A tape is just a folder containing your configuration overrides and a simple `tape.toml` manifest.

### Directory Structure
```text
my-tape/
├── tape.toml         # Tape manifest
├── hypr/             # Overrides ~/.config/hypr
│   └── hyprland.conf
├── waybar/           # Overrides ~/.config/waybar
│   └── config.jsonc
└── nvim/             # Overrides ~/.config/nvim
    └── init.lua
```

### `tape.toml` Specification
```toml
[tape]
name = "neon-dream"
desc = "Vibrant synthwave theme for Hyprland + Waybar"
dependencies = ["hyprland", "waybar"]

[hooks]
# Commands executed automatically right after the atomic swap
insert = [
    "hyprctl reload",
    "pkill -SIGUSR2 waybar",
    "notify-send 'Tapes' 'Switched to Neon Dream!'",
]

# Commands executed automatically when ejecting this tape
eject = [
    "hyprctl reload",
    "pkill -SIGUSR2 waybar",
]
```

> **Note:** Any files or folders outside of what your tape provides (like `~/.config/google-chrome`, `~/.config/pulse`, etc.) remain completely untouched and active!

---

## 🔬 Under the Hood: How Atomicity Works

1. **Staging (`abb`)**: Tapes creates `~/.config.tapes` on the exact same filesystem as `~/.config`. It creates symlinks for untouched directories and layers the tape's overrides over them.
2. **Validation**: Tapes verifies that every symlink target resolves. If any target is missing or malformed, the staging directory is wiped clean and execution halts—leaving your real config untouched.
3. **The Kernel Swap**: Tapes invokes Linux `renameat2(None, "~/.config", None, "~/.config.tapes", RENAME_EXCHANGE)`. The Linux kernel swaps the directory inode pointers instantaneously in a single operation.
4. **Safety Ownership**: A `.tapes-managed` marker and state record in `~/.local/share/tapes/` ensure Tapes never touches unmanaged configurations.

---

## 🤝 Contributing

Contributions are welcome! Please check out [Contributing.md](Contributing.md) for architectural guidelines, our strict never-nesting style rules, and commit message formatting.

---

## 📄 License

Distributed under the [MIT License](LICENSE). Made with ❤️ for the Linux r/unixporn and dotfiles community.
