<div align="center">

# c3
![GitHub top language](https://img.shields.io/github/languages/top/nimaaskarian/c3?color=orange)
![AUR version](https://img.shields.io/aur/version/c3?logo=archlinux)
![2024-04-15-132950-snap](https://github.com/nimaaskarian/c3/assets/88832088/f5b38ef0-a37c-4949-9209-8abae5df4775)


A crossplatform to-do list app that uses and extends [calcurse](https://www.calcurse.org/)'s format, to be a tree like to-do list with both sub-dependencies and notes.

[Getting started](#getting-started) •
[Installation](#installation) •
[Usage](#usage)
</div>


## Installation
### Compiling it yourself
You can simply compile this like any rust application with the commands below
```bash
git clone https://github.com/nimaaskarian/c3
cd c3
cargo build --release
sudo cp target/release/c3 /usr/bin/
```

If you use **Arch linux**, You can install [c3 from AUR](https://aur.archlinux.org/packages/c3). Installation using yay would be
```bash
yay -S c3
```
### Using a pre-built release
You can check out [releases](https://github.com/nimaaskarian/c3/releases).
Also if you use **Arch linux**, you can install a pre-built binary [from AUR](https://aur.archlinux.org/packages/c3-bin). Installation using yay would be
```bash
yay -S c3-bin
```

## Usage
### Interactive mode
The default mode of the app is TUI mode. Keybinds are vim-like. Here they are:

| key | action |
|---|---|
| a | add todo to bottom|
| A | add todo to top|
| e | edit todo |
| E | edit todo (move cursor to start) |
| ! | toggle show done |
| 0-9 | set todo priority |
| j | go down in todo list |
| k | go up in todo list |
| g | go top of todo list |
| G | go bottom of todo list |
| J | increase todo priority |
| K | decrease todo priority |
| d | toggle daily |
| W | toggle weekly |
| S | set custom schedule |
| m | Set todo as a reminder
| D | delete todo |
| > | add todo note |
| i | increase day done |
| o | decrease day done |
| t | add todo dependency |
| l | go in depedency/add todo dependency |
| h | go back to parent |
| T | delete todo dependency/note |
| x | cut todo to clipboard |
| y | yank todo to clipboard |
| p | paste todo from clipboard |
| P | enable module |
| / | search current list for todo |
| ? | search the whole tree for todo |
| O | open nnn file picker for choosing a file to append to current list
| n | search next |
| N | search previous |
| w | write changes to file |
| R | read from file (discard changes)|
#### Modules
TUI mode has a section called module. You can develop modules, and assign methods to Module trait methods.
A very simple example has been done by default for [potato-c](https://github.com/nimaaskarian/potato-c).

Keybinds that modules can use are **space, H, L, comma, period, +, -, s, r**

### Non interactive mode
For command line arguments and such, run `c3 -h` to see full usage.
