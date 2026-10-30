<!-- PROJECT HEADER -->
<br />
<div align="center">

<h1 align="center">Tapes V2</h1>

<img src="media/icon.svg" alt="Tapes logo" width="200">

An atomic dotfile switcher for ricing.
Swap your whole setup in one command, get it back in one command.

Rewritten to be safer, simpler and truly atomic

<a href="https://github.com/futurekismo-oss/tapes-v2/issues/new">Report Bug</a>
&middot;
<a href="https://github.com/futurekismo-oss/tapes-v2/issues/new">Request Feature</a>

</div>

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust Edition: 2024](https://img.shields.io/badge/Rust-2024-orange.svg)](Cargo.toml)
[![Release](https://img.shields.io/github/v/release/futurekismo-oss/tapes-v2?color=brightgreen)](https://github.com/futurekismo-oss/tapes-v2/releases)

---

## Why Tapes?

If you stare at your desktop for longer than zero hours, you have personal configs of every app you use in `~/.config`. Your desktop decorations and app settings all technically live in there, which means changing how your entire desktop looks is done the manual way... or with **tapes**.

A **tape** is someone's whole `~/.config` setup — or just the parts they want to share — packed into one folder you can plug in with a single command.

Most dotfile managers (like GNU Stow or manual symlink managers) operate file-by-file. If something crashes midway, your `~/.config` is left in a broken, half-swapped state.

Tapes does the opposite: it builds your new config tree off to the side, then swaps the old and new trees in one single kernel operation. That means:

- **No half-states** — `~/.config` is either your old setup or the new one, never in between.
- **Instant rollback** — `tapes-v2 eject` puts things back exactly how they were, atomically.
- **Your files are never touched** — the original config is never modified, overwritten, or deleted.

That's the whole idea. If you want to know how the swap actually works, see [INTERNALS.md](INTERNALS.md).

In day-to-day terms: keep as many tapes in your library as you like and insert and eject them at will — the workflow is like running `matugen`, except instead of regenerating colors it swaps entire config trees. And insertion is not a full reset: only the configs a tape actually provides are overridden, everything else from your old `~/.config` carries over. A tape that ships just a `quickshell/` config swaps only that — your Chromium profile stays logged in and your `nvim` config stays yours, no re-login, no expenses.

---

## Quickstart

Install a tape, insert it into your desktop, and eject when you want your old setup back:

```bash
# 1. Install downloads or copies a tape into the library (~/.tapes/). It doesn't touch anything else yet
tapes-v2 install https://github.com/username/some_dots --name demo

# Or from a local folder:
# tapes-v2 install ~/my-dots --name demo

# 2. See what the tape contains and what it overrides
tapes-v2 show demo

# 3. Insert the tape (swaps your config and runs reload hooks)
tapes-v2 insert demo

# 4. Check what's active
tapes-v2 status

# 5. Revert back like nothing ever happened
tapes-v2 eject
```

> Tip: add `alias tapes="tapes-v2"` to your `~/.bashrc` or `~/.zshrc` to use `tapes` as a shortcut.

> **Before inserting a tape you didn't write, read it.** Hooks are arbitrary shell commands that run as your user the moment a swap lands — `hyprctl reload` is the friendly example, but a `tape.toml` can just as easily contain `rm -rf`. The swap itself is reversible; a hook command is not. `tapes-v2 show <name>` prints every hook a tape runs, and `insert --no-hooks` lets you test a tape without executing any of its commands first.

---

## Installation

Tapes is Linux-only — the whole trick is a kernel syscall, so macOS and BSD are out of scope.

### Option 1: Install script (Arch and other Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/futurekismo-oss/tapes-v2/main/install.sh | bash
```

### Option 2: Nix / NixOS

```bash
nix run github:futurekismo-oss/tapes-v2 -- --help
# or add it to your profile:
nix profile install github:futurekismo-oss/tapes-v2
```

### Option 3: cargo binstall
```bash
# it is required to specify the version until tapes leaves beta
cargo binstall tapes-v2@0.1.2-beta
```

### Option 4: From source

```bash
git clone https://github.com/futurekismo-oss/tapes-v2.git
cd tapes-v2
cargo install --path .
```

---

## Commands

| Command | What it does |
|---------|--------------|
| `tapes-v2 install <source>` | Downloads or copies a tape into your library (`~/.tapes/`) |
| `tapes-v2 show [name]` | Lists all installed tapes, or inspects one |
| `tapes-v2 insert [name]` | Swaps a tape into `~/.config` and runs its insert hooks |
| `tapes-v2 eject` | Swaps your original config back and runs its eject hooks |
| `tapes-v2 status [-v]` | Shows the active tape, where it came from, and its targets |

Handy flags on top of the above:

```bash
--dry-run    # show the plan without touching anything
--no-hooks   # skip the hook commands from the manifest (e.g. hyprctl reload)
install -b   # install a specific branch, tag, or commit
--path       # insert straight from any directory, skipping the library
-q           # quiet output
```

The short version:

```bash
# Clone someone's rice and try it
tapes-v2 install user/awesome-rice

tapes-v2 install user/awesome-rice --dir dots # or from a specific folder in the repo
tapes-v2 install user/awesome-rice --branch dots # or branch

tapes-v2 insert awesome-rice

# Don't like it? One command and nothing ever happened
tapes-v2 eject
```

---

## Creating Your Own Tape

A tape is just a folder with your config overrides and a `tape.toml` manifest.

The top level of a tape mirrors the top level of `~/.config` — a folder named `hypr` in the tape overrides `~/.config/hypr`, and so on.

```text
my-tape/
├── tape.toml         # manifest
├── hypr/             # overrides ~/.config/hypr
│   └── hyprland.conf
├── waybar/           # overrides ~/.config/waybar
│   └── config.jsonc
└── nvim/             # overrides ~/.config/nvim
    └── init.lua
```

`tape.toml`:

```toml
[tape]
name = "neon-dream"
desc = "Vibrant synthwave theme for Hyprland + Waybar"
dependencies = ["hyprland", "waybar"]

[hooks]
# Commands run right after the swap
insert = [
    "hyprctl reload",
    "pkill -SIGUSR2 waybar",
    "notify-send 'Tapes' 'Switched to Neon Dream!'",
]

# Commands run after the tape is ejected
eject = [
    "hyprctl reload",
    "pkill -SIGUSR2 waybar",
]
```

Any configs your tape doesn't provide (Chrome profiles, `pulse`, whatever else lives in `~/.config`) stay exactly as they are.

---

## Roadmap

- A TUI
- Colored output
- More features

---

## Contributing

Contributions are welcome. See [Contributing.md](Contributing.md) for the guidelines.

The internals of how Tapes works live in [INTERNALS.md](INTERNALS.md) — worth a read before you touch the code.

---

## License

Distributed under the [MIT License](LICENSE).
