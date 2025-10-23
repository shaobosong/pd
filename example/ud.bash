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
