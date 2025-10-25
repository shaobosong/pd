# pd

An interactive parent directory navigator.

This tool allows a user to interactively select a component of the current
working directory's path. It then prints the selected parent path to stdout,
which is intended to be captured by a shell function to quickly change
directories "up" the tree.

## Features

- Vim and Emacs style keybindings.
- Mouse click and scroll wheel support for navigation.
- Cross-platform compatibility (Windows, macOS, Linux).
- Guaranteed terminal state restoration on exit via the RAII pattern.

## Build

```sh
cargo build --release
```

## Installation

```sh
cargo install --path .
```

## Usage

### Configuration

You can customize the keybindings by setting the `PD_KEYMAP` environment variable.
- `PD_KEYMAP=vim`: Use Vim-style keybindings. (Default)
- `PD_KEYMAP=emacs`: Use Emacs-style keybindings.

### Keybindings & Controls

#### Shared Controls (All Modes)

| **Key(s)**           | **Action**                                    |
| -------------------- | --------------------------------------------- |
| `Enter`              | Confirm selection and print directory.        |
| `q`, `Esc`, `Ctrl-c` | Quit.                                         |
| `Ctrl-z`             | Suspend the process (Unix-like systems only). |

#### Vim Mode (Default)

| **Key(s)**                   | **Action**                                                 |
| ---------------------------- | ---------------------------------------------------------- |
| `h`, `k`, `b`, `Left Arrow`  | Move selection left.                                       |
| `l`, `j`, `w`, `Right Arrow` | Move selection right.                                      |
| `^`, `H`, `Home`             | Move selection to the first part.                          |
| `0`                          | Move selection to the first part.                          |
| `$`, `L`, `End`              | Move selection to the last part.                           |
| `M`                          | Move selection to the middle part.                         |
| `<number><key>`              | Prepend a count to a motion (e.g., `2h` moves left twice). |

#### Emacs Mode

| **Key(s)**                       | **Action**                        |
| -------------------------------- | --------------------------------- |
| `Ctrl-b`, `Alt-b`, `Left Arrow`  | Move selection left.              |
| `Ctrl-f`, `Alt-f`, `Right Arrow` | Move selection right.             |
| `Ctrl-a`, `Home`                 | Move selection to the first part. |
| `Ctrl-e`, `End`                  | Move selection to the last part.  |

#### Mouse Controls

| **Action**            | **Behavior**                            |
| --------------------- | --------------------------------------- |
| **Hover**             | Move the selection under the cursor.    |
| **Left Click**        | Confirm selection and print directory.  |
| **Right Click**       | Quit.                                   |
| **Scroll Up/Left**    | Move selection left.                    |
| **Scroll Down/Right** | Move selection right.                   |

## Example

- bash
```bash
function ud() {
  local d; d=$(pd) && cd "$d"
}
```

- fish
```fish
function ud
  set -l d (pd); and cd $d
end
```

- nushell
```nushell
def --env --wrapped ud [...rest:string] {
  cd $'(pd -- ...$rest | str trim)'
}
```

- posix
```sh
ud() {
  d=$(pd) && cd "$d"
}
```

- powershell
```pwsh
function ud {
  $d = pd; if ($LASTEXITCODE -eq 0 -and $d) { Set-Location $d }
}
```

- zsh
```zsh
function ud() {
  local d; d=$(pd) && cd "$d"
}
```

## LICENSE

MIT
