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

## LICENSE
MIT
