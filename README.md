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
3. **Safe, Instant Rollback**: Run `tapes-v2 eject`, and your original configuration is atomically swapped right back. Your real files are never modified, overwritten, or deleted.

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
# 1. Install a tape (from GitHub or a local directory)
tapes-v2 install futurekismo-oss/tapes-v2 --name demo

# Or install your own local folder:
# tapes-v2 install ~/my-dotfiles --name my-tape

# 2. Inspect the tape and see what it overrides
tapes-v2 show demo

# 3. Insert the tape (atomically switches your config and runs reload hooks)
tapes-v2 insert demo

# 4. Check active status
tapes-v2 status

# 5. Revert back to your original setup like nothing happened
tapes-v2 eject
```

> 💡 **Tip:** Add `alias tape="tapes-v2"` to your `~/.bashrc` or `~/.zshrc` to use `tape` as a convenient shortcut!

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

### `tapes-v2 install <SOURCE>`
Downloads or copies a tape into your local library (`~/.tapes/`).
- **Public Repositories**: Use GitHub shorthand (`owner/repo`) or HTTPS URLs (`https://github.com/...`). Clones instantly without login prompts.
- **Private Repositories**: Use SSH URLs (`git@github.com:owner/repo.git`) to clone using your local SSH keys.
- **Local Folders**: Use any folder path (`~/my-dots`, `./path`).
- Automatically generates an example `tape.toml` with commented hooks if the source lacks one.

```bash
# 1. Public GitHub shorthand
tapes-v2 install user/hypr-dots

# 2. Private repository via SSH (uses your ~/.ssh keys)
tapes-v2 install git@github.com:user/private-dots.git --name work-theme

# 3. Full Git URL with custom name and specific branch
tapes-v2 install https://github.com/user/dots.git --name catppuccin -b dev

# 4. Local directory
tapes-v2 install ~/Downloads/cool-theme --name cool-theme
```

---

### `tapes-v2 insert [NAME]`
Atomically swaps the specified tape into your active `~/.config`.
- Resolves all targets: folders/files provided by the tape override your current config; any configs the tape does not touch are preserved.
- Automatically executes `[hooks.insert]` (e.g. window manager/bar reloads).

```bash
# Insert a tape from your ~/.tapes library
tapes-v2 insert demo

# Test with a dry run (shows the plan without touching the filesystem)
tapes-v2 insert demo --dry-run

# Insert without running hook scripts
tapes-v2 insert demo --no-hooks
```

---

### `tapes-v2 eject`
Atomically restores your original `~/.config` and cleanly deletes the staging directory.
- Runs `[hooks.eject]` commands to reset running desktop components.

```bash
# Revert back to default configuration
tapes-v2 eject

# Dry-run test
tapes-v2 eject --dry-run
```

---

### `tapes-v2 status`
Displays the currently active tape, its source path, and target link records.

```bash
tapes-v2 status

# Detailed verbose view
tapes-v2 status -v
```

---

### `tapes-v2 show [NAME]`
Lists all installed tapes in `~/.tapes`, or inspects a specific tape's manifest, description, and hooks.

```bash
# List all tapes in ~/.tapes
tapes-v2 show

# Inspect manifest details for a tape
tapes-v2 show demo
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
