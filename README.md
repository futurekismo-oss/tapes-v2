# Tapes v2

A small CLI that swaps dotfile sets in and out of `~/.config` without trashing the rest of it.

I wrote v1 and it worked on my machine and barely anywhere else. State lived in a hand parsed text file, backups were a single `.bak` that blew up on the second insert, and the symlink code pointed at the wrong prefix for `.config` layouts. I scrapped it. This is the rewrite.

Core rule. Only the targets a tape declares get touched. Everything else stays put.

```sh
tape insert dms
tape status
tape eject dms
```

## How it works

A tape is a folder. Usually it lives in `~/.local/share/tapes/<name>`, but any directory works with `--path`.

```text
~/.local/share/tapes/dms/
  tape.toml
  .config/hypr/...
  .config/quickshell/dms/...
```

Insert does this, in order.

1. Reads the manifest, checks the names, resolves the target list.
2. Checks that required binaries exist on `PATH`.
3. Takes the lock so two `tape` runs can't interleave.
4. Scans for conflicts before changing anything.
5. Moves each conflicting real file or dir at the destination into a timestamped backup.
6. Creates each symlink atomically, temp link plus rename.
7. Writes one small JSON record per target, then updates `active.json`.
8. Runs your post hooks.

Eject reverses it. It reads the records, removes only symlinks that still point at the recorded source, moves backups back, deletes the records, updates `active.json`, runs your eject hooks. If you repointed a symlink by hand after insert, eject leaves it alone and warns on stderr. It will not delete a link it does not own.

## Install and build

You need Rust 1.85 or newer.

```sh
cargo build --release
sudo install -m 755 target/release/tape /usr/local/bin/tape
```

Or run it without installing.

```sh
cargo run -- insert dms --dry-run
```

Nix users get a dev shell with the toolchain wired up.

```sh
nix develop
```

Releases strip symbols and use thin LTO. Debug builds leave that off.

## Tape layouts that work

I wanted foreign rices to work without repackaging. Three shapes resolve.

- `.config/` at the top. `my-rice/.config/hypr` maps to `~/.config/hypr`.
- `dots/.config/` at the top. This is what the ii forks use. `fork/dots/.config/quickshell/ii` maps to `~/.config/quickshell/ii`.
- Bare files. A clone with `hypr/` and `quickshell/` at the top maps each entry straight into `~/.config/`.

If a `tape.toml` exists it wins. If not, the directory name becomes the tape name and everything resolves by the rules above. That is how you try a random GitHub rice with zero prep.

## The manifest

Full example. Every section except `[tape]` name is optional.

```toml
schema_version = 2

[tape]
name = "dms"
version = "0.1.0"
desc = "DMS plus hypr"
provides = ["shell"]
requires = []

[dependencies]
binaries = ["hyprland", "quickshell"]
packages = ["quickshell-git"]

[targets]
"hypr" = "~/.config/hypr"
"quickshell/dms" = "~/.config/quickshell/dms"

[hooks]
pre_insert = []
post_insert = ["hyprctl reload"]
pre_eject = []
post_eject = []
```

What each part does.

- `schema_version` stays at 2. The loader rejects anything else in state files.
- `provides` and `requires` are plain labels for now. Nothing enforces them yet. I kept them in the schema so composition has somewhere to live later.
- `dependencies.binaries` must exist on `PATH` or insert stops before changing anything. `dependencies.packages` never installs. It only shows up in logs so you know what the rice author intended.
- `[targets]` maps a path inside the tape to a destination. The destination understands `~/` and `$HOME/`, plus `~/.config/` which follows `XDG_CONFIG_HOME`. The source must stay relative and must not contain `..`.
- Hooks run with `sh -c`, in order. Old v1 names still parse. `insert` means `post_insert` and `eject` means `post_eject`. Old `tape.dependencies = [...]` merges into `dependencies.binaries`.

Smallest manifest that still does something.

```toml
[tape]
name = "mine"
version = "0.1.0"
desc = ""
```

## Commands

Global flag `--json` works on insert, eject, status, show, and validate. Scripts should use it. Humans should skip it.

### insert

```sh
tape insert dms
tape insert --path ~/refs/ii-p3drovfx --dry-run
tape insert dms --path ~/my-fork --reinsert --no-hooks
```

Flags.

- `--path DIR` inserts from anywhere instead of the library.
- `--reinsert` lets you replace a tape that is already active or take a destination owned by another active tape.
- `--dry-run` prints the plan and changes nothing. No lock side effects, no hooks, no records.
- `--no-hooks` skips all hooks.

When you pass both a name and `--path`, the name is the label stored in `active.json` and the path is the content. When you pass only `--path`, the manifest name, or the directory name if there is no manifest, becomes the label.

Inserting an already active tape without `--reinsert` is a no-op if every link already points at the right source. If anything drifted, it errors and tells you to pass `--reinsert`.

### eject

```sh
tape eject
tape eject dms
tape eject dms --dry-run
```

With no name and exactly one active tape, it ejects that tape. With several active, it stops and asks you to name one. That check exists because ejecting the wrong rice and restoring the wrong backup would ruin your afternoon.

### status

```sh
tape status
tape status --json
```

Shows the active stack in insert order plus every recorded symlink, its source, and its backup if one exists. Empty state prints one line and exits zero.

### show

```sh
tape show
tape show dms
```

No name lists the library, sorted. A name prints version, description, directory, and resolved targets. Broken names get skipped in the list rather than failing the whole command.

### snapshot

```sh
tape snapshot mine --entry kitty --entry foot
```

Copies the given entries out of your live config into a new library tape under `.config/`, then writes a minimal `tape.toml` if none exists. Entries are relative to `~/.config`, repeatable, and must not be absolute or contain `..`. If the source entry is a symlink, the snapshot follows it and copies content, so the tape stays self contained.

I require `--entry` on purpose. Copying all of `~/.config` blindly drags in browsers and caches. Name what you mean to keep.

### validate

```sh
tape validate ~/refs/ii-p3drovfx
```

Parses the manifest, resolves targets, reports missing binaries. Changes nothing. Run this before you insert a stranger's rice.

### doctor

```sh
tape doctor
```

Checks home and XDG paths, library writability, `sh` and `git` presence, and compositor variables. Missing Wayland vars only warn. This command is safe to run in CI sandboxes and on machines without Hyprland.

## Files on disk

```text
~/.config/                       your live config, partly symlinks while a tape is in
~/.local/share/tapes/            the library
~/.local/share/tapes/<name>/     one tape
~/.local/share/tapes/active.json stack of active tape names plus schema version
~/.local/share/tapes/records/    one JSON file per managed symlink
~/.local/share/tapes/backups/    timestamped originals, named <time>_<tape>__<slug>
~/.local/share/tapes/.lock       lock file, content is meaningless
```

`XDG_CONFIG_HOME` and `XDG_DATA_HOME` redirect the first two roots. The library creates missing dirs on first run.

A record looks like this.

```json
{
  "schema_version": 2,
  "tape": "dms",
  "symlink_path": "/home/you/.config/hypr",
  "symlink_target": "/home/you/.local/share/tapes/dms/.config/hypr",
  "backup_path": "/home/you/.local/share/tapes/backups/20260915-120301_dms__hypr",
  "installed_at": "2026-09-15T12:03:01Z"
}
```

Writes go to a temp file, get fsynced, then rename over the destination. The parent dir fsync is best effort. If it fails you get a warning on stderr, not a failed insert, because the content is already durable by then.

## Walkthrough, caelestia to an end4 fork

Say you run caelestia today and want to try the ii fork for an evening.

```sh
tape snapshot caelestia --entry hypr --entry quickshell/caelestia
tape validate ~/refs/ii-p3drovfx
tape insert --path ~/refs/ii-p3drovfx --dry-run
tape insert --path ~/refs/ii-p3drovfx
hyprctl reload
```

Look around. If you hate it:

```sh
tape eject
hyprctl reload
```

Your old files come back from the timestamped backup. Your chromium profile, keys, and anything the fork never declared were never moved in the first place.

## Hooks and dependencies

Keep hooks short and loud. Each line runs with `sh -c`. Empty lines get skipped. A failing hook stops the command with the exit code attached. Insert records are already written before post hooks run, so a failed post hook does not roll back symlinks. I chose that because silently unwinding a half hooked desktop felt worse than telling you exactly what failed.

```toml
[hooks]
post_insert = ["hyprctl reload", "notify-send 'rice on'"]
post_eject = ["hyprctl reload"]
```

Binary deps fail closed. If `hyprland` is not on `PATH`, insert stops before the lock section mutates anything. Package names are advisory only. v2 will never call paru, yay, or pkexec on its own. If a rice needs packages, install them yourself or wire that into a hook you wrote and can read.

## What v2 does not do

No TUI. No package installs. No process killing. No background daemon. No central registry. Hooks are the escape hatch for all of that, and they stay visible in the manifest.

Multiple tapes can be active at once as long as their destinations do not collide. A collision without `--reinsert` is an error naming both tapes and the path. That is the whole composition story for now.

## Testing and CI

The library takes explicit paths, never reads env directly, so tests run in tempdirs.

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

CI runs those three plus a sandbox smoke test that sets `HOME`, `XDG_CONFIG_HOME`, and `XDG_DATA_HOME` into a temp dir, inserts a demo tape, checks the symlink, ejects, and checks it is gone. Nothing touches a real home, no compositor needed, no network after vendoring.

## When something goes wrong

- `another tape process holds the lock` means a second run is active or a previous run died mid op. Wait, or check for a stray `tape` process, then retry. The lock file itself is fine to leave in place.
- `target conflict at ...` names the owner and the requester. Either eject the owner first or retry with `--reinsert`.
- `already active (use --reinsert)` means the label is in `active.json` but links drifted. Diff with `tape status`, then reinsert if that is what you want.
- `missing dependency binary` lists what was not found. Install it or drop it from the manifest.
- `no active tape, nothing to eject` is exactly what it says.
- A record with a schema other than 2 errors loudly instead of guessing. I would rather stop than migrate state I do not understand.

## v1 notes for upgraders

Tape dirs carry over. Manifests carry over, including the old `insert`/`eject` hook names and `tape.dependencies` lists. State does not carry over. v1's `current-tape` file is ignored. Eject or back up under v1 first if you are mid rice, then start clean on v2.
