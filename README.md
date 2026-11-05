<br />
<div align="center">

<h1 align="center">Tapes</h1>

<img src="media/icon.svg" alt="Tapes logo" width="200">

An atomic dotfile switcher for Linux and ricing.
Swap your whole setup in one command and get it back in one command.

Rewritten to be safer, simpler and truly atomic

<a href="https://github.com/futurekismo-oss/tapes/issues/new">Report Bug</a>
&middot;
<a href="https://github.com/futurekismo-oss/tapes/issues/new">Request Feature</a>
&middot;
<a href="https://discord.gg/TVjsGzFAur">Join the Discord</a>

</div>

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust Edition: 2024](https://img.shields.io/badge/Rust-2024-orange.svg)](Cargo.toml)
[![Release](https://img.shields.io/github/v/release/futurekismo-oss/tapes?color=brightgreen)](https://github.com/futurekismo-oss/tapes/releases)

---

## Tapes in Action

https://github.com/user-attachments/assets/6791a91b-a3be-429b-9fd0-adc34fc93b08

---

## Quickstart

Either package a `~/.config` as a tape (put in it a folder, move to `~/.tapes` and add a tape.toml inside), or install one directly from a repo. Then, you can easily insert the tape to try it out and eject it just as easily to return back to your old `~/.config`.

```bash
# You found a cyberpunk dotfiles in a public repo
tapes install https://github.com/some_user/cyberpunk --name cyberpunk 
# if its in a specific branch or subdir in the repo, utilize the --branch and --dir subcommands

# Insert a tape sitting in ~/.tapes
tapes insert cyberpunk # tapes expects the tape in ~/.tapes/cyberpunk

# Wanna see its properties and hooks before insertion?
tapes show cyberpunk

# List all the tapes in ~/tapes
tapes show

# Is a tape currently inserted?
tapes status

# Wanna go back to your old config?
tapes eject
```

It's as simple as that!
A tape.toml is meant to look like this:

```toml
[tape]
name = "cyberpunk" # required
desc = "A cool cyberpunk rice"
dependencies = ["hyprland", "quickshell"] # just useful data for the user, tapes does nothing with it as of now

[hooks]
insert = [
  "hyprctl reload",
  "qs kill",
  "waybar &",   "notify-send 'Tapes' 'Switched to cyberpunk rice'"
]

eject = [
  "hyprctl reload",
  "pkll waybar",
  # Tapes runs commands in the current shell session, so try to move them to the background
  "qs -c caelestia -d",  
  "notify-send 'Tapes' 'Back to caelestia'",
]

# install = [] # exists btw
```

Tapes expects the directory tree to look like this:

```text
cyberpunk/
├── tape.toml         
├── hypr/             
│   └── hyprland.conf
├── waybar/           
│   └── config.jsonc
└── nvim/            
    └── init.lua
```


Some handy flags for tapes:

```bash
--dry-run # shows the plan without touching any files
--no-hooks # skip running any hooks
-p, --path # insert a tape straight out any folder
-q, --queit # quiet output
-v, --verbose # verbose output
-l, --logging # show logs, useful for reporting issues and debugging

-V, --version # display the current version
```

---

## Installation

**Install script** (grabs the latest release, re-run it to update)

```bash
curl -fsSL https://raw.githubusercontent.com/futurekismo-oss/tapes/main/install.sh | bash
```
Run `tapes --version` to ensure it worked. Make sure to add `~/.local/bin` to your `$PATH`.

**Cargo**

```bash
cargo install tapes@v0.1.5-beta
cargo binstall tapes@v0.1.5-beta # prebuilt binary instead of compiling
```

Try to add `~/.cargo/bin` to your `$PATH` if `tapes --version` doesn't work

**Nix**

```bash
nix run github:futurekismo-oss/tapes
nix profile install github:futurekismo-oss/tapes
```

**From source**

```bash
git clone https://github.com/futurekismo-oss/tapes
cd tapes
cargo install --path .
```

---

Tapes supports tab completion. Just add ``source <(COMPLETIONS=zsh tapes)`` to your shell init (`.zshrc`, or `.bashrc`). Replace `zsh` with your chosen shell. It supports `zsh`, `fish`, `bash`, `elvish` and `nushell`

> **Tip**: If you find the name `tapes` annoying you can alias it to `tapes`, the binary can't be renamed tapes as it's taken on [crates.io](https://crates.io/search?q=tapes)


## Why Use Tapes?
If you're like me and a big unixporn fan, you've definitely found yourself wanting to try new rices, however the worry of trashing your old config holds you back. Install scripts barely care; they frequentely override previous configs or just backup the entire config folder and make a new one, forcing you go to back and manually move files to fix the mess. 

Tapes aims to solve this by making trying out new rices and reverting back to your old config as frictionless as possible as while being safe and fast. Other dotfile managers have come close, but don't really give this feeling of pop in a config and pop it out as fast (this is where the name came from). The initial implementation (v1) was hasty and buggy. It was a prototype.

Now, let's explain the terms you'll come across while using this program.

**Tape**: A tape is ~/.config folder, a rice, a dotfile; whatever you choose to call it. It contains folders like `quickshell` and `hypr`. Tapes can take this config folder and swap its `targets` or folders with your own, allowing you to use the example quickshell and hypr config in this tape as your config without losing things like `chromium` keys (which are stored in `~/.config/chromium` for some reason).

**Target**: A target is a folder in a tape, a `quickshell` folder, or `hypr` folder, but a target can also be a file e.g `.zshrc`, `starship.toml`. These targets tell tapes the config you wanna insert into your `~/.config` folder.

**Atomic**: You've heard me say it a lot. It means there is no in-between; when tapes insert a tapes, it takes the targets from your `~/.config` folder, asks the tape which config it provides, and then builds an entire valid config folder called `~/.config.tapes` made up of symlinks from the tape and your previous config. It then uses the renameat linux syscall to swap your `~/.config` with `~/.config.tapes` (meaning your old config becomes `~/.config.tapes` and vice versa), and as there is no in-between state where your config is missing, it can call itself atomic, since anything before the swap can be easily rolled back by deleting `~/.config.tapes`.

**Hooks**: Shell commands in the tape.toml. Tapes can run them after insert, after eject or after install. *Make sure to read these if a repo provides its own tape.toml, even though it shouldn't.*

---

## Acknowledgements

Thanks to these wonderful individuals for

> [erotic-erox](https://github.com/exotic-erox) for calling out some lifecycle bugs on version 0.1.2-beta

> [LilaccFlowerr](https://github.com/LilaccFlowerr) for providing the tapes showcase video and hopefully a gui in the future

> [saksham_kumar](https://github.com/saksham_kumar) for being a contributor and writing the beloved install command

> [Mvk](https://github.com/Mvkweb) for providing lots of encouragement and support

> [Clover](https://github.com/Cloverthestars) for providing the most encouragement and support, He encouraged the rewrite of tapes turning it from a broken mess to an actual usable utility

> [Marv](https://github.com/shreudi) for encouraging me to build tapes after i pitched the initial idea, without him, tapes would still be a random afternoon thought


---

## Roadmap

- [ ] A TUI or GUI
- [ ] Colored output
- [ ] More features

---

## Contributing

Contributions are welcome. See [Contributing.md](Contributing.md) for the guidelines.

---

## License

Distributed under the [MIT License](LICENSE).


