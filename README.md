# Tapes-v2

A atomic, per-file dotfiles switcher

## TODO
- [x] show command
- [x] insert command
- [x] eject command
- [x] status command
- [x] install command - install a tape to ~/.tapes from any repo
- [ ] unit and integration tests

## Dependencies
- Linux
- Some dotfiles (subject to change)
- Git - for install command

## Installation
Nix
```bash
nix run github:futurekismo-oss/tapes-v2
```
or this command
```bash
bash <(curl -sSL https://raw.githubusercontent.com/futurekismo-oss/tapes-v2/refs/heads/main/install.sh)
```
