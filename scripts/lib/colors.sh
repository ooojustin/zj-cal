# colors.sh - ANSI color utils
# Justin Garofolo <justin@garofolo.net>
# https://github.com/ooojustin/dotfiles/blob/master/lib/colors.sh

# Generate raw escape sequence from code
_esc() { printf '\033[%sm' "$1"; }

# Wrap text with color (optional 'bold' as 3rd arg)
_color() {
    local code="$1" text="$2" bold=""
    [[ "${3:-}" == "1" ]] && bold="1;"
    printf "\033[${bold}${code}m%s\033[0m" "$text"
}

# Color codes (standard)
_C_RST=0
_C_BLD=1
_C_BLK=30
_C_RED=31
_C_GRN=32
_C_YLW=33
_C_BLU=34
_C_MAG=35
_C_CYN=36
_C_WHT=37
_C_GRY=90

# Color codes (bright)
_C_BRED=91
_C_BGRN=92
_C_BYLW=93
_C_BBLU=94
_C_BMAG=95
_C_BCYN=96
_C_BWHT=97

# Raw ANSI codes (standard)
_RST=$(_esc $_C_RST)
_BLD=$(_esc $_C_BLD)
_BLK=$(_esc $_C_BLK)
_RED=$(_esc $_C_RED)
_GRN=$(_esc $_C_GRN)
_YLW=$(_esc $_C_YLW)
_BLU=$(_esc $_C_BLU)
_MAG=$(_esc $_C_MAG)
_CYN=$(_esc $_C_CYN)
_WHT=$(_esc $_C_WHT)
_GRY=$(_esc $_C_GRY)

# Raw ANSI codes (bright)
_BRED=$(_esc $_C_BRED)
_BGRN=$(_esc $_C_BGRN)
_BYLW=$(_esc $_C_BYLW)
_BBLU=$(_esc $_C_BBLU)
_BMAG=$(_esc $_C_BMAG)
_BCYN=$(_esc $_C_BCYN)
_BWHT=$(_esc $_C_BWHT)

# Color helpers
_bold() { _color $_C_BLD "$1"; }
_black() { _color $_C_BLK "$1" "${2:-}"; }
_red() { _color $_C_RED "$1" "${2:-}"; }
_green() { _color $_C_GRN "$1" "${2:-}"; }
_yellow() { _color $_C_YLW "$1" "${2:-}"; }
_blue() { _color $_C_BLU "$1" "${2:-}"; }
_magenta() { _color $_C_MAG "$1" "${2:-}"; }
_cyan() { _color $_C_CYN "$1" "${2:-}"; }
_white() { _color $_C_WHT "$1" "${2:-}"; }
_gray() { _color $_C_GRY "$1" "${2:-}"; }

# Bright variants
_bred() { _color $_C_BRED "$1" "${2:-}"; }
_bgreen() { _color $_C_BGRN "$1" "${2:-}"; }
_byellow() { _color $_C_BYLW "$1" "${2:-}"; }
_bblue() { _color $_C_BBLU "$1" "${2:-}"; }
_bmagenta() { _color $_C_BMAG "$1" "${2:-}"; }
_bcyan() { _color $_C_BCYN "$1" "${2:-}"; }
_bwhite() { _color $_C_BWHT "$1" "${2:-}"; }

# Usage examples
#
#   echo "$(_green 'success')"
#   echo "$(_red 'error'): something went wrong"
#   echo "$(_bold 'important')"
#
# Bold colors:
#
#   echo "$(_cyan 'normal') vs $(_cyan 'bold' 1)"
#   echo "$(_red 'warning' 1)"
#
# Combine with regular text:
#
#   echo "Status: $(_green 'OK')"
#   echo "Build $(_red 'FAILED' 1) at step 3"
#
# Raw codes (for sed, etc.):
#
#   sed "s/ERROR/${_RED}ERROR${_RST}/g"
#   sed "s/WARN/${_BYLW}WARN${_RST}/g"
