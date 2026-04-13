#!/bin/bash
# ═══════════════════════════════════════════════════════════════════════════════
#  Foxix Terminal - Instalador
#  https://github.com/jephersonRD/foxix
# ═══════════════════════════════════════════════════════════════════════════════

if [ ! -t 0 ]; then
  TEMP_SCRIPT=$(mktemp)
  cat > "$TEMP_SCRIPT"
  chmod +x "$TEMP_SCRIPT"
  bash "$TEMP_SCRIPT" "$@" < /dev/tty
  rm -f "$TEMP_SCRIPT"
  exit $?
fi

if [ -z "$BASH_VERSION" ]; then
  exec bash "$0" "$@"
fi

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
WHITE='\033[1;37m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

INSTALL_DIR="$HOME/.local/foxix"
REPO_URL="https://github.com/jephersonRD/foxix-terminal"
BIN_LINK="/usr/local/bin/foxix"
CONFIG_DIR="$HOME/.config/foxix"

LANG_CHOICE="es"

t() {
  local key="$1"
  case "$LANG_CHOICE" in
    "es")
      case "$key" in
        "select_lang") echo "Selecciona tu idioma / Select your language:" ;;
        "spanish") echo "Español" ;;
        "english") echo "Inglés" ;;
        "opt_install") echo "Instalar" ;;
        "opt_repair") echo "Reparar" ;;
        "opt_remove") echo "Desinstalar" ;;
        "opt_exit") echo "Salir" ;;
        "select_opt") echo "Selecciona una opción:" ;;
        "invalid_opt") echo "Opción no válida. Intenta de nuevo." ;;
        "detecting") echo "Detectando sistema..." ;;
        "detected") echo "Sistema detectado:" ;;
        "checking_deps") echo "Verificando dependencias..." ;;
        "installing") echo "Instalando..." ;;
        "missing_deps") echo "Dependencias faltantes:" ;;
        "installing_deps") echo "Instalando dependencias..." ;;
        "downloading") echo "Descargando Foxix..." ;;
        "download_ok") echo "Descarga completada" ;;
        "compiling") echo "Compilando Foxix (esto puede tomar unos minutos)..." ;;
        "compile_ok") echo "Compilación completada" ;;
        "install_complete") echo "¡Instalación completada!" ;;
        "launching") echo "Para ejecutar Foxix, usa: foxix" ;;
        "repairing") echo "Reparando instalación..." ;;
        "repair_ok") echo "Reparación completada" ;;
        "removing") echo "Eliminando Foxix..." ;;
        "remove_ok") echo "Foxix eliminado" ;;
        "goodbye") echo "¡Hasta luego!" ;;
        "already_installed") echo "Foxix ya está instalado." ;;
        "not_installed") echo "Foxix no está instalado." ;;
        "root_error") echo "No ejecutes este script como root." ;;
        "deps_ok") echo "Todas las dependencias OK" ;;
        "create_config") echo "Creando configuración..." ;;
        "config_ok") echo "Configuración creada" ;;
        "make_link") echo "Creando enlace simbólico..." ;;
        "link_ok") echo "Enlace creado" ;;
        "missing_rust") echo "Rust no está instalado. Instálalo primero." ;;
        "uninstall_failed") echo "No se pudo eliminar (requiere permisos sudo)." ;;
        "check_deps") echo "Verificando..." ;;
        "install_deps_question") echo "¿Deseas instalar las dependencias faltantes? (s/n)" ;;
        *) echo "$key" ;;
      esac
      ;;
    "en")
      case "$key" in
        "select_lang") echo "Select your language / Selecciona tu idioma:" ;;
        "spanish") echo "Spanish" ;;
        "english") echo "English" ;;
        "opt_install") echo "Install" ;;
        "opt_repair") echo "Repair" ;;
        "opt_remove") echo "Uninstall" ;;
        "opt_exit") echo "Exit" ;;
        "select_opt") echo "Select an option:" ;;
        "invalid_opt") echo "Invalid option. Try again." ;;
        "detecting") echo "Detecting system..." ;;
        "detected") echo "System detected:" ;;
        "checking_deps") echo "Checking dependencies..." ;;
        "installing") echo "Installing..." ;;
        "missing_deps") echo "Missing dependencies:" ;;
        "installing_deps") echo "Installing dependencies..." ;;
        "downloading") echo "Downloading Foxix..." ;;
        "download_ok") echo "Download completed" ;;
        "compiling") echo "Compiling Foxix (this may take a few minutes)..." ;;
        "compile_ok") echo "Compilation completed" ;;
        "install_complete") echo "Installation completed!" ;;
        "launching") echo "To run Foxix, use: foxix" ;;
        "repairing") echo "Repairing installation..." ;;
        "repair_ok") echo "Repair completed" ;;
        "removing") echo "Removing Foxix..." ;;
        "remove_ok") echo "Foxix removed" ;;
        "goodbye") echo "Goodbye!" ;;
        "already_installed") echo "Foxix is already installed." ;;
        "not_installed") echo "Foxix is not installed." ;;
        "root_error") echo "Do not run this script as root." ;;
        "deps_ok") echo "All dependencies OK" ;;
        "create_config") echo "Creating configuration..." ;;
        "config_ok") echo "Configuration created" ;;
        "make_link") echo "Creating symbolic link..." ;;
        "link_ok") echo "Link created" ;;
        "missing_rust") echo "Rust is not installed. Install it first." ;;
        "uninstall_failed") echo "Could not remove (requires sudo permissions)." ;;
        "check_deps") echo "Checking..." ;;
        "install_deps_question") echo "Do you want to install missing dependencies? (y/n)" ;;
        *) echo "$key" ;;
      esac
      ;;
  esac
}

show_banner() {
  clear
  echo ""
  echo -e "${MAGENTA}${BOLD}"
  echo "  ███████╗ ██████╗ ██╗  ██╗██╗██╗  ██╗"
  echo "  ██╔════╝██╔═══██╗╚██╗██╔╝██║╚██╗██╔╝"
  echo "  █████╗  ██║   ██║ ╚███╔╝ ██║ ╚███╔╝ "
  echo "  ██╔══╝  ██║   ██║ ██╔██╗ ██║ ██╔██╗ "
  echo "  ██║     ╚██████╔╝██╔╝ ██╗██║██╔╝ ██╗"
  echo "  ╚═╝      ╚═════╝ ╚═╝  ╚═╝╚═╝╚═╝  ╚═╝"
  echo ""
  echo -e "${NC}"
  echo -e "    ${DIM}https://github.com/jephersonRD/foxix-terminal${NC}"
  echo ""
}

detect_distro() {
  echo -e "\n${CYAN}$(t "detecting")${NC}"
  sleep 0.5

  if [ -f /etc/os-release ]; then
    . /etc/os-release
    DISTRO_NAME="$NAME"
    DISTRO_ID="$ID"
    DISTRO_LIKE="$ID_LIKE"
  elif [ -f /etc/lsb-release ]; then
    . /etc/lsb-release
    DISTRO_NAME="$DISTRIB_DESCRIPTION"
    DISTRO_ID="$DISTRIB_ID"
    DISTRO_LIKE=""
  else
    DISTRO_NAME="Unknown"
    DISTRO_ID="unknown"
    DISTRO_LIKE=""
  fi

  echo -e "${GREEN}$(t "detected") ${WHITE}${DISTRO_NAME}${NC}"
  echo ""

  case "$DISTRO_ID" in
    arch|manjaro|endeavouros|garuda|artix)
      PKG_MANAGER="pacman"
      PKG_INSTALL="sudo pacman -S --noconfirm"
      PKG_UPDATE="sudo pacman -Syu --noconfirm"
      ;;
    ubuntu|debian|linuxmint|pop|elementary|zorin)
      PKG_MANAGER="apt"
      PKG_INSTALL="sudo apt install -y"
      PKG_UPDATE="sudo apt update"
      ;;
    fedora|nobara)
      PKG_MANAGER="dnf"
      PKG_INSTALL="sudo dnf install -y"
      PKG_UPDATE="sudo dnf check-update"
      ;;
    opensuse*|suse)
      PKG_MANAGER="zypper"
      PKG_INSTALL="sudo zypper install -y"
      PKG_UPDATE="sudo zypper refresh"
      ;;
    void)
      PKG_MANAGER="xbps"
      PKG_INSTALL="sudo xbps-install -y"
      PKG_UPDATE="sudo xbps-install -Syu"
      ;;
    *)
      if echo "$DISTRO_LIKE" | grep -qi "arch"; then
        PKG_MANAGER="pacman"
        PKG_INSTALL="sudo pacman -S --noconfirm"
        PKG_UPDATE="sudo pacman -Syu --noconfirm"
      elif echo "$DISTRO_LIKE" | grep -qi "debian\|ubuntu"; then
        PKG_MANAGER="apt"
        PKG_INSTALL="sudo apt install -y"
        PKG_UPDATE="sudo apt update"
      elif echo "$DISTRO_LIKE" | grep -qi "fedora\|rhel"; then
        PKG_MANAGER="dnf"
        PKG_INSTALL="sudo dnf install -y"
        PKG_UPDATE="sudo dnf check-update"
      else
        PKG_MANAGER="unknown"
        PKG_INSTALL=":"
        PKG_UPDATE=":"
      fi
      ;;
  esac
}

check_deps() {
  echo -e "${CYAN}$(t "checking_deps")${NC}\n"

  local missing=()
  local deps=()

  case "$PKG_MANAGER" in
    pacman)
      deps=("rust" "freetype2" "wayland")
      ;;
    apt)
      deps=("rustc" "libfreetype-dev" "libwayland-dev")
      ;;
    dnf)
      deps=("rust" "freetype-devel" "wayland-devel")
      ;;
    zypper)
      deps=("rust" "freetype2-devel" "wayland-devel")
      ;;
    xbps)
      deps=("rust" "freetype" "wayland")
      ;;
    *)
      deps=("rust" "freetype" "wayland")
      ;;
  esac

  for dep in "${deps[@]}"; do
    if command -v "$dep" &> /dev/null || [ -f "/usr/bin/$dep" ] || [ -f "/usr/local/bin/$dep" ]; then
      echo -e "${GREEN}  ✓ $dep${NC}"
    else
      echo -e "${RED}  ✗ $dep${NC}"
      missing+=("$dep")
    fi
  done

  if [ ${#missing[@]} -gt 0 ]; then
    echo -e "\n${YELLOW}$(t "missing_deps") ${missing[*]}${NC}"
    echo -e "${CYAN}$(t "install_deps_question")${NC}"
    read -rp "> " install_deps

    if [[ "$install_deps" == "s" || "$install_deps" == "S" || "$install_deps" == "y" || "$install_deps" == "Y" ]]; then
      echo -e "${CYAN}$(t "installing_deps")${NC}\n"
      $PKG_UPDATE 2>/dev/null || true
      for pkg in "${missing[@]}"; do
        $PKG_INSTALL "$pkg" 2>/dev/null || {
          echo -e "${RED}Error installing ${pkg}${NC}"
        }
      done
      echo -e "\n${GREEN}✓ Dependencias instaladas${NC}"
    fi
  else
    echo -e "\n${GREEN}$(t "deps_ok")${NC}"
  fi
}

spinner_pid=0
spinner_chars=("⠋" "⠙" "⠹" "⠸" "⠼" "⠴" "⠦" "⠧" "⠇" "⠏")

start_spinner() {
  local message="$1"
  tput civis
  (
    local i=0
    while true; do
      printf "\r${DIM}%s ${spinner_chars[$i]} ${message}   ${NC}" "${spinner_chars[$i]}"
      i=$(( (i + 1) % ${#spinner_chars[@]} ))
      sleep 0.1
    done
  ) &
  spinner_pid=$!
}

stop_spinner() {
  if [ $spinner_pid -ne 0 ]; then
    kill $spinner_pid 2>/dev/null || true
    spinner_pid=0
    tput cnorm
    printf "\r                                                                 \r"
  fi
}

download_and_build() {
  mkdir -p "$INSTALL_DIR"

  if [ -d "$INSTALL_DIR/foxix" ]; then
    rm -rf "$INSTALL_DIR/foxix"
  fi

  start_spinner "Descargando Foxix..."
  if git clone --depth 1 "$REPO_URL" "$INSTALL_DIR/foxix" 2>/dev/null; then
    sleep 0.5
    stop_spinner
    echo -e "${GREEN}✓ $(t "download_ok")${NC}"
  else
    stop_spinner
    echo -e "${RED}Error downloading project${NC}"
    exit 1
  fi

  start_spinner "Compilando Foxix..."
  cd "$INSTALL_DIR/foxix"
  
  local compile_log=$(cargo build --release 2>&1)
  
  if [ -f "$INSTALL_DIR/foxix/target/release/foxix" ]; then
    sleep 0.5
    stop_spinner
    echo -e "${GREEN}✓ $(t "compile_ok")${NC}"
  else
    stop_spinner
    echo -e "${RED}Compilation failed${NC}"
    echo "$compile_log" | tail -10
    exit 1
  fi
}

create_config() {
  echo -e "${CYAN}$(t "create_config")${NC}"

  mkdir -p "$CONFIG_DIR"

  if [ ! -f "$CONFIG_DIR/foxix.conf" ]; then
    cat > "$CONFIG_DIR/foxix.conf" << 'EOF'
font_family             JetBrains Mono Nerd Font
font_size               12
background_opacity      0.78
window_padding_width    25
shell                   
scrollback_lines        10000
EOF
  fi

  echo -e "${GREEN}$(t "config_ok")${NC}"
}

make_link() {
  echo -e "${CYAN}$(t "make_link")${NC}"

  if [ -f "$BIN_LINK" ]; then
    sudo rm "$BIN_LINK"
  fi

  sudo ln -s "$INSTALL_DIR/foxix/target/release/foxix" "$BIN_LINK"

  echo -e "${GREEN}$(t "link_ok")${NC}"
}

do_install() {
  show_banner
  echo -e "${BOLD}${MAGENTA}  ┌─────────────────────────────────────┐${NC}"
  echo -e "${BOLD}${MAGENTA}  │${NC}  ${WHITE}$(t "opt_install")${NC}                             ${BOLD}${MAGENTA}│${NC}"
  echo -e "${BOLD}${MAGENTA}  └─────────────────────────────────────┘${NC}"
  echo ""

  if [ -f "$BIN_LINK" ]; then
    echo -e "${YELLOW}$(t "already_installed")${NC}"
    echo -e "${DIM}Usa la opción 2 para reparar.${NC}"
    read -rp "Presiona Enter para continuar..."
    return
  fi

  detect_distro
  check_deps

  if ! command -v cargo &> /dev/null; then
    echo -e "${RED}$(t "missing_rust")${NC}"
    read -rp "Presiona Enter para continuar..."
    return
  fi

  download_and_build
  create_config
  make_link

  echo ""
  echo -e "${GREEN}${BOLD}  ╔═══════════════════════════════════════╗${NC}"
  echo -e "${GREEN}${BOLD}  ║                                       ║${NC}"
  echo -e "${GREEN}${BOLD}  ║     ✓ $(t "install_complete")          ║${NC}"
  echo -e "${GREEN}${BOLD}  ║                                       ║${NC}"
  echo -e "${GREEN}${BOLD}  ╚═══════════════════════════════════════╝${NC}"
  echo ""
  echo -e "${WHITE}$(t "launching")${NC}"
  echo ""

  read -rp "Presiona Enter para continuar..."
}

do_repair() {
  show_banner
  echo -e "${BOLD}${YELLOW}  ┌─────────────────────────────────────┐${NC}"
  echo -e "${BOLD}${YELLOW}  │${NC}  ${WHITE}$(t "opt_repair")${NC}                               ${BOLD}${YELLOW}│${NC}"
  echo -e "${BOLD}${YELLOW}  └─────────────────────────────────────┘${NC}"
  echo ""

  if [ ! -d "$INSTALL_DIR/foxix" ]; then
    echo -e "${RED}$(t "not_installed")${NC}"
    echo -e "${DIM}Usa la opción 1 para instalar.${NC}"
    read -rp "Presiona Enter para continuar..."
    return
  fi

  echo -e "${CYAN}$(t "repairing")${NC}\n"

  detect_distro
  check_deps

  cd "$INSTALL_DIR/foxix"
  cargo build --release 2>&1 | tail -5

  chmod +x "$INSTALL_DIR/foxix/target/release/foxix"

  if [ ! -L "$BIN_LINK" ]; then
    make_link
  fi

  echo -e "\n${GREEN}$(t "repair_ok")${NC}"
  read -rp "Presiona Enter para continuar..."
}

do_remove() {
  show_banner
  echo -e "${BOLD}${RED}  ┌─────────────────────────────────────┐${NC}"
  echo -e "${BOLD}${RED}  │${NC}  ${WHITE}$(t "opt_remove")${NC}                           ${BOLD}${RED}│${NC}"
  echo -e "${BOLD}${RED}  └─────────────────────────────────────┘${NC}"
  echo ""

  if [ ! -d "$INSTALL_DIR" ] && [ ! -f "$BIN_LINK" ]; then
    echo -e "${YELLOW}$(t "not_installed")${NC}"
    read -rp "Presiona Enter para continuar..."
    return
  fi

  echo -e "${YELLOW}¿Estás seguro? (s/n)${NC}"
  read -rp "> " confirm

  if [[ "$confirm" != "s" && "$confirm" != "S" ]]; then
    echo -e "${DIM}Operación cancelada${NC}"
    read -rp "Presiona Enter para continuar..."
    return
  fi

  echo -e "\n${CYAN}$(t "removing")${NC}"

  rm -rf "$INSTALL_DIR"

  if [ -L "$BIN_LINK" ]; then
    sudo rm "$BIN_LINK" 2>/dev/null || {
      echo -e "${RED}$(t "uninstall_failed")${NC}"
    }
  fi

  echo -e "${GREEN}$(t "remove_ok")${NC}"
  read -rp "Presiona Enter para continuar..."
}

show_menu() {
  show_banner
  echo -e "  ${BOLD}${WHITE}$(t "select_opt")${NC}"
  echo ""
  echo -e "  ${CYAN}1)${NC} ${GREEN}$(t "opt_install")${NC}"
  echo -e "  ${CYAN}2)${NC} ${YELLOW}$(t "opt_repair")${NC}"
  echo -e "  ${CYAN}3)${NC} ${RED}$(t "opt_remove")${NC}"
  echo -e "  ${CYAN}4)${NC} ${DIM}$(t "opt_exit")${NC}"
  echo ""
  echo -ne "  ${BOLD}> ${NC}"
}

select_language() {
  clear
  echo ""
  echo -e "${MAGENTA}${BOLD}"
  echo "  ███████╗ ██████╗ ██╗  ██╗██╗██╗  ██╗"
  echo "  ██╔════╝██╔═══██╗╚██╗██╔╝██║╚██╗██╔╝"
  echo "  █████╗  ██║   ██║ ╚███╔╝ ██║ ╚███╔╝ "
  echo "  ██╔══╝  ██║   ██║ ██╔██╗ ██║ ██╔██╗ "
  echo "  ██║     ╚██████╔╝██╔╝ ██╗██║██╔╝ ██╗"
  echo "  ╚═╝      ╚═════╝ ╚═╝  ╚═╝╚═╝╚═╝  ╚═╝"
  echo ""
  echo -e "${NC}"
  echo -e "    ${DIM}https://github.com/jephersonRD/foxix-terminal${NC}"
  echo ""
  echo -e "  ${WHITE}$(t "select_lang")${NC}"
  echo ""
  echo -e "  ${CYAN}1)${NC} ${WHITE}$(t "spanish")${NC}"
  echo -e "  ${CYAN}2)${NC} ${WHITE}$(t "english")${NC}"
  echo ""
  echo -ne "  ${BOLD}> ${NC}"

  while true; do
    read -r lang_choice
    case $lang_choice in
      1) LANG_CHOICE="es"; break ;;
      2) LANG_CHOICE="en"; break ;;
      *) echo -e "  ${RED}$(t "invalid_opt")${NC}" ;;
    esac
  done
}

main() {
  if [ "$EUID" -eq 0 ]; then
    echo -e "${RED}$(t "root_error")${NC}"
    exit 1
  fi

  select_language

  while true; do
    show_menu
    read -r option
    case $option in
      1) do_install ;;
      2) do_repair ;;
      3) do_remove ;;
      4)
        echo ""
        echo -e "${GREEN}$(t "goodbye")${NC}"
        echo ""
        exit 0
        ;;
      *)
        echo -e "  ${RED}$(t "invalid_opt")${NC}"
        sleep 1
        ;;
    esac
  done
}

main