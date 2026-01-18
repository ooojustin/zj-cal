set dotenv-load

root := justfile_directory()
colors := justfile_directory() / "scripts/lib/colors.sh"
ics_url := env_var_or_default("ZJ_CAL_ICS_URL", "")
test_port := "8088"

build *args:
    cargo build {{args}}

# Run tests (requires native target)
test *args:
    #!/usr/bin/env bash
    set -euo pipefail
    source "{{colors}}"

    native_target=$(rustc -vV | grep host | cut -d' ' -f2)
    echo "$(_cyan 'Target:' 1) $native_target"
    cargo test --target "$native_target" {{args}}

# Build/run the plugin in debug mode, for development.
# Examples:
#   just run
#   just run -t                  # use test calendar (requires `just serve-ics`)
#   just run -c "foo=bar"        # adds extra config
#   just run -f                  # floating window
run *args:
    #!/usr/bin/env bash
    set -eo pipefail
    source "{{colors}}"

    ics_url="{{ics_url}}"
    extra_args=()
    debug_ics=""

    # parse args
    args=({{args}})
    while [[ ${#args[@]} -gt 0 ]]; do
        case "${args[0]}" in
            -t|--test)
                ics_url="http://localhost:{{test_port}}/zj-cal-test.ics"
                debug_ics=1
                args=("${args[@]:1}")  # shift past flag
                ;;
            -c|--configuration)
                extra_config="${args[1]}"  # grab value after flag
                args=("${args[@]:2}")  # shift past flag and value
                ;;
            *)
                extra_args+=("${args[0]}")  # pass through to zellij
                args=("${args[@]:1}")  # shift past
                ;;
        esac
    done

    # build debug binary
    ZJ_CAL_DEBUG_ICS="$debug_ics" cargo build

    # build plugin config str
    config="ics_url=$ics_url"
    [[ -n "${extra_config:-}" ]] && config="$config,$extra_config"

    plugin_path="file:{{root}}/target/wasm32-wasip1/debug/zj-cal.wasm"
    display_config=$(echo "$config" | sed -E 's|ics_url=[^,]*|ics_url=[REDACTED]|g')

    if [[ ${#extra_args[@]} -gt 0 ]]; then
        echo -e "$(_cyan 'Running:' 1)\n  zellij plugin -s -c \"$display_config\" ${extra_args[*]} -- \"$plugin_path\""
        zellij plugin -s -c "$config" "${extra_args[@]}" -- "$plugin_path"
    else
        echo -e "$(_cyan 'Running:' 1)\n  zellij plugin -s -c \"$display_config\" -- \"$plugin_path\""
        zellij plugin -s -c "$config" -- "$plugin_path"
    fi

# Generate test ICS file
gen-ics:
    #!/usr/bin/env bash
    source "{{colors}}"
    ./scripts/gen-test-ics.sh > /tmp/zj-cal-test.ics
    echo "$(_green 'Generated:' 1) /tmp/zj-cal-test.ics"

# Serve test ICS on localhost
serve-ics: gen-ics
    #!/usr/bin/env bash
    source "{{colors}}"
    echo "$(_cyan 'Serving:' 1) http://localhost:{{test_port}}/zj-cal-test.ics"
    echo "$(_cyan 'Run:' 1) just run -t"
    cd /tmp && python3 -m http.server {{test_port}}

# Watch plugin logs
# Use -a/--all to show all Zellij logs (not just zj-cal)
logs *args:
    #!/usr/bin/env bash
    set -euo pipefail
    source "{{colors}}"

    log_path="${TMPDIR:-/tmp}/zellij-$(id -u)/zellij-log/zellij.log"
    filter="grep --line-buffered zj-cal |"
    label="zj-cal logs"

    for arg in {{args}}; do
        case "$arg" in
            -a|--all) filter=""; label="all logs" ;;
        esac
    done

    echo -e "$(_cyan "Streaming $label from:" 1)\n  $log_path"
    eval "tail -f -n 0 '$log_path' | $filter sed -u \
        -e 's/ERROR/${_RED}ERROR${_RST}/g' \
        -e 's/WARN/${_YLW}WARN${_RST}/g' \
        -e 's/INFO/${_GRN}INFO${_RST}/g' \
        -e 's/DEBUG/${_CYN}DEBUG${_RST}/g'"
