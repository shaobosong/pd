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
```sh
vim ud.bash
```

```bash
# ██╗   ██╗██████╗
# ██║   ██║██╔══██╗
# ██║   ██║██║  ██║
# ██║   ██║██║  ██║
# ╚██████╔╝██████╔╝
#  ╚═════╝ ╚═════╝ ud: Interactively change directory upwards

ud() {
    local clear_line=$(tput el) # Clear line from cursor to end
    local navigator ret retcode target_dir error
    navigator="$(command -v pd)"
    ret="$(${navigator})"; retcode=$?
    if test "$retcode" -ne 0; then
        error="$ret"
        test -n "$error" &&
            printf "\r${clear_line}%s" "${error}" >&2 ||
            printf "\r${clear_line}"
    else
        target_dir="$ret"
        printf "\r${clear_line}%s\n" "${target_dir}"
        cd "$target_dir"
    fi
    return "$retcode"
}
```

```sh
source /path/ud.bash
ud
```

## LICENSE
MIT
