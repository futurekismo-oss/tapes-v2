# Internals

How Tapes works under the hood. The [README](README.md) stays simple; this page is for anyone modifying the tool, debugging it, or just curious about the atomic swap.

---

## The Problem

Dotfile managers usually work file-by-file. That means a switch is a *sequence* of changes, and anything that interrupts that sequence (a crash, a full disk, an error) leaves `~/.config` half-swapped. There is no undo, because the "before" state was destroyed step by step.

Tapes avoids sequences entirely: it never touches `~/.config` until it can replace the whole directory in a single step.

## Staging

An insert never modifies your real config. Instead it builds the *entire* target tree in a sibling directory:

```text
~/.config            ~/.config.tapes        (both, right before the swap)
├── nvim/            ├── nvim/     → symlink into the tape (tape's version)
├── hypr/            ├── hypr/     → symlink into the tape (tape's version)
├── waybar/          ├── waybar/   → symlink into the tape (tape's version)
└── chrome/          ├── chrome/   → symlink to itself (preserved)
                     └── .tapes-managed
```

Two kinds of entries end up in the staging directory:

- **Overrides** — every file or folder the tape provides is symlinked from the staging tree into the tape directory. You are always using the tape's own files, which stay in place (so they remain editable, git-versioned, etc.).
- **Preserved** — every entry already in `~/.config` that the tape does *not* provide gets a symlink that points at itself from within the staging tree. Nothing is copied, so nothing can drift.

The `.tapes-managed` marker is written into the staging tree. This is how Tapes can later prove the tree it is standing in front of is one it built itself.

## The Swap

When the staging tree is complete, Tapes calls one syscall:

```text
renameat2(AT_FDCWD, ~/.config, AT_FDCWD, ~/.config.tapes, RENAME_EXCHANGE)
```

`RENAME_EXCHANGE` atomically swaps the two entries. It is not "rename one away, rename the other in" — it is a single operation in the kernel, and either it happens or it doesn't.

After the swap:

- `~/.config` is the tree Tapes just staged (the tape is live).
- `~/.config.tapes` now holds your **original** config, untouched, exactly as it was.

Rollback is the same trick in reverse: `eject` exchanges the two directories again, giving you the original back, and then removes the leftover staging tree (only a pile of symlinks — the real files live elsewhere).

Because the swap is a single step, there are no crash windows. Your config is always one of the two states, never a mixture of both. If the swap fails, neither directory has been touched.

## Insert Flow

1. If a tape is already active, Tapes asks before ejecting it first.
2. The tape is resolved from the library name, `--path`, or a `./tape.toml` in the current directory.
3. Tapes refuses to run if `~/.config` itself is a symlink (the swap depends on it being a real directory).
4. The staging tree is built:
   - aborts if `~/.config.tapes` already exists (a dry-run leftover, or a previous insert that died mid-flight)
5. The atomic swap happens.
6. The new state is recorded in `~/.local/share/tapes/state.json`.
7. `[hooks.insert]` commands run one at a time through `$SHELL -c`, in listed order.
   - a failing hook stops the remaining ones, but does **not** un-swap; the tape is already live

## Eject Flow

1. Tapes validates before touching anything:
   - `~/.config` and `~/.config.tapes` must both exist
   - `~/.config` must contain the `.tapes-managed` marker
   - if not, it refuses to eject ("not managed by tapes")
2. The state is cleared and the directories are swapped back.
3. The staging directory is removed.
4. `[hooks.eject]` commands run.

## On-Disk Layout

```text
~/.tapes/                          tape library
├── my-rice/                       installed tapes, as-is
└── ...
~/.local/share/tapes/
└── state.json                     active tape name, path, and symlink records
~/.config.tapes                    created at insert; holds your original config while a tape is active; gone after eject
~/.config
└── .tapes-managed                 written after a successful swap
```

`state.json` is written through a temp file + rename, so the file is never seen half-written.

## Dry Run

`insert --dry-run` builds the staging tree and prints the plan, then stops — no swap, no state change, your real config is untouched. The leftover `~/.config.tapes` must be removed (`rm -r ~/.config.tapes`) before a real insert will run.

## When Things Go Wrong

Rule zero: the swap is the only destructive step, and it is all-or-nothing. Everything before it builds, everything after it cleans up. The failure cases:

| Situation | What happened | Remedy |
|-----------|---------------|--------|
| `status` shows no tape, but `~/.config.tapes` exists | a `--dry-run` leftover, or an insert that died before the swap | inspect it; if it holds nothing unique, `rm -r ~/.config.tapes` and try again |
| Tapes refuses to eject ("not managed by tapes") | the `.tapes-managed` marker is missing from `~/.config` | if `status` says a tape is active and you still want out, `touch ~/.config/.tapes-managed` and retry |
| Eject swap failed | nothing is touched; your current view and the original config are both intact | retry, or swap manually (below) |
| Eject finished but complained about removing `~/.config.tapes` | the swap already happened; only the cleanup failed | `rm -r ~/.config.tapes` |

Manual recovery in a pinch (if it ever dies at 3 in the morning):

```bash
# eject swap failed, your original config is still waiting in .config.tapes:
mv ~/.config ~/.config.tape-junk
mv ~/.config.tapes ~/.config
```

## Manifest Reference

```toml
[tape]
name = "my-rice"              # required
desc = "My awesome setup"     # required
dependencies = ["hyprland", "waybar"]  # optional; declared, not yet enforced

[hooks]                       # optional
insert = ["hyprctl reload"]   # run after insert
eject = ["hyprctl reload"]    # run after eject
```

Hooks are plain shell strings — write them exactly as you would type them in a terminal. Each runs through your login shell (`$SHELL -c`), one at a time, in the order listed. A non-zero exit stops the remaining hooks, but the swap itself is never undone.

Trust note for people installing tapes you didn't write: hooks run with your full user privileges and Tapes deliberately adds no sandbox around them. Read a tape's `[hooks]` sections (`tapes-v2 show <name>`) before trusting it — the atomic swap is reversible, the commands it triggers are not.

---

## Notes For Editors

- State is a single JSON file with no locking. Never run two inserts concurrently.
- The `Commit` subcommand exists in the CLI but is not implemented.
- `dependencies` is parsed from the manifest but never enforced (yet).
