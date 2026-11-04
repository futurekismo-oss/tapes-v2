# FAQ

### How do i use tapes?

Well read the README and the FAQ and join the discord so you can be told to read the README and FAQ


### What is a tape? 

Well go back and read the readme but if you're too lazy for that

```
Tape: A tape is ~/.config folder, a rice, a dotfile; whatever you choose to call it. It contains folders like quickshell and hypr. Tapes can take this config folder and swap its targets or folders with your own, allowing you to use the example quickshell and hypr config in this tape as your config without losing things like chromium keys (which are stored in ~/.config/chromium for some reason).
```

### How can i make a tape? 

Easy, get some configs put them in a folder and slap on a tape.toml; an example tape.toml can be found in the readme. To convert a local config dir into a tape, i suggest you use the install command

```bash
tapes-v2 install /path/to/config --name custom-name
```

tapes will copy your the dir to `~/.tapes` and add a example `tape.toml`. If however you wanna install a popular rice or dotfile like caelestia as a tape also use the install command but provide a github link instead of a directory

```bash
tapes-v2 install https://github.com/link-to-repo.git --name custom-name
```
you can use the `--dir` arg to get a sub-directory in the repo and the `--branch` arg for fetch a specific branch from the repo


### I used the install command. What now??

Well you now have a tape, go to `~/.tapes` find the name of the tape you just created and edit the hooks in `tape.toml`. Hooks are commands you can run on insert and eject, they run in your current shell session so if you need them to run in the background, try to disown the command or run as daemons ( for qs, its `qs -d` ). You can use hooks to send notifs that the tape has swapped, run fastfetch, reload quickshell, etc..





