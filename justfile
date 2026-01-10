set dotenv-load

root := justfile_directory()
ics_url := env_var_or_default("ZJ_CAL_ICS_URL", "")

# ANSI color codes
export C_RESET := '\x1b[0m'
export C_BOLD := '\x1b[1m'
export C_RED := '\x1b[31m'
export C_GREEN := '\x1b[32m'
export C_YELLOW := '\x1b[33m'
export C_CYAN := '\x1b[36m'

build *args:
    cargo build {{args}}

# Build/run the plugin in debug mode, for development.
# Examples:
#   just run
#   just run -c "foo=bar"        # adds extra config
#   just run -f                  # floating window
#   just run -c "foo=bar" -f     # both
run *args: build
    #!/usr/bin/env bash
    set -eo pipefail

    config="ics_url={{ics_url}}"
    extra_args=()

    args=({{args}})
    while [[ ${#args[@]} -gt 0 ]]; do
        case "${args[0]}" in
            -c|--configuration)
                config="$config,${args[1]}"
                args=("${args[@]:2}")
                ;;
            *)
                extra_args+=("${args[0]}")
                args=("${args[@]:1}")
                ;;
        esac
    done

    plugin_path="file:{{root}}/target/wasm32-wasip1/debug/zj-cal.wasm"
    display_config=$(echo "$config" | sed -E 's|ics_url=[^,]*|ics_url=[REDACTED]|g')

    if [[ ${#extra_args[@]} -gt 0 ]]; then
        echo -e "${C_BOLD}${C_CYAN}Running:${C_RESET}\n  zellij plugin -s -c \"$display_config\" ${extra_args[*]} -- \"$plugin_path\""
        zellij plugin -s -c "$config" "${extra_args[@]}" -- "$plugin_path"
    else
        echo -e "${C_BOLD}${C_CYAN}Running:${C_RESET}\n  zellij plugin -s -c \"$display_config\" -- \"$plugin_path\""
        zellij plugin -s -c "$config" -- "$plugin_path"
    fi

# Watch plugin logs
# Use -a/--all to show all Zellij logs (not just zj-cal)
logs *args:
    #!/usr/bin/env bash
    set -euo pipefail

    log_path="${TMPDIR:-/tmp}/zellij-$(id -u)/zellij-log/zellij.log"
    filter="grep --line-buffered zj-cal |"
    label="zj-cal logs"

    for arg in {{args}}; do
        case "$arg" in
            -a|--all) filter=""; label="all logs" ;;
        esac
    done

    echo -e "${C_BOLD}${C_CYAN}Streaming ${label} from:${C_RESET}\n  $log_path"
    eval "tail -f -n 0 '$log_path' | $filter sed -u \
        -e 's/ERROR/${C_RED}ERROR${C_RESET}/g' \
        -e 's/WARN/${C_YELLOW}WARN${C_RESET}/g' \
        -e 's/INFO/${C_GREEN}INFO${C_RESET}/g' \
        -e 's/DEBUG/${C_CYAN}DEBUG${C_RESET}/g'"
