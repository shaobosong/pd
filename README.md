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

## Usage
The keymap can be set to Emacs mode by setting the `PD_KEYMAP` environment
variable to `emacs`. It defaults to Vim mode otherwise.

## Build
```sh
cargo build --release
```

## Installation
```sh
cargo install --path .
```

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
