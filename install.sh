#!/usr/bin/env bash
# ============================================================================
# S# Language Installer for Linux/macOS
# ============================================================================
# Compiles the interpreter and installs it to ~/.local/bin/ssharp.
#
# Usage:
#   git clone https://github.com/neo-zero23/S-Sharp.git
#   cd S-Sharp
#   ./install.sh
#
# Exit codes: 0 = success, 1 = error.
# ============================================================================
set -u

APP_NAME="ssharp"
INSTALL_DIR="$HOME/.local/bin"
BIN_SRC="target/release/$APP_NAME"
BIN_DST="$INSTALL_DIR/$APP_NAME"

step() { printf '\n==> %s\n' "$1"; }
ok()   { printf '  [OK] %s\n' "$1"; }
warn() { printf '  [AVISO] %s\n' "$1"; }
fail() { printf '  [ERROR] %s\n' "$1" >&2; exit 1; }

# Always run from the repo root (where this script lives).
cd "$(dirname "$0")"

# --- 1. Check for cargo ------------------------------------------------------
step "Verificando cargo..."
if ! command -v cargo >/dev/null 2>&1; then
    fail "Cargo no encontrado. Instalá Rust desde https://rustup.rs"
fi
ok "cargo encontrado: $(command -v cargo)"

# --- 2. Build release --------------------------------------------------------
step "Compilando $APP_NAME (release)..."
if ! cargo build --release; then
    fail "Falló la compilación. Revisá los errores de cargo arriba."
fi
[ -x "$BIN_SRC" ] || fail "No se generó el binario $BIN_SRC."
ok "compilación exitosa: $BIN_SRC"

# --- 3. Install binary -------------------------------------------------------
step "Instalando en $BIN_DST..."
mkdir -p "$INSTALL_DIR" || fail "No se pudo crear $INSTALL_DIR."
cp -f "$BIN_SRC" "$BIN_DST" || fail "No se pudo copiar el binario."
chmod +x "$BIN_DST" || fail "No se pudo dar permiso de ejecución."
ok "binario instalado."

# --- 4. Verify installation --------------------------------------------------
step "Verificando instalación..."
if ! "$BIN_DST" --version >/dev/null 2>&1; then
    fail "El binario instalado no responde a --version."
fi
ok "$("$BIN_DST" --version)"

# --- 5. PATH check -----------------------------------------------------------
step "Verificando PATH..."
if [[ ":$PATH:" == *":$INSTALL_DIR:"* ]]; then
    ok "~/.local/bin ya está en tu PATH."
else
    warn "~/.local/bin NO está en tu PATH."
    printf '  Agregá esta línea a tu ~/.bashrc o ~/.zshrc y abrí una terminal nueva:\n'
    printf '    export PATH="$HOME/.local/bin:$PATH"\n'
fi

# --- 6. Optional: .ssharp file association -----------------------------------
step "Asociación de archivos .ssharp (opcional)..."
if ! command -v xdg-mime >/dev/null 2>&1; then
    warn "xdg-mime no disponible, se omite la asociación de archivos."
else
    MIME_DIR="$HOME/.local/share/mime/packages"
    mkdir -p "$MIME_DIR" || warn "no se pudo crear $MIME_DIR."
    cat > "$MIME_DIR/ssharp.xml" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="text/x-ssharp">
    <comment>S# script</comment>
    <glob pattern="*.ssharp"/>
  </mime-type>
</mime-info>
EOF
    if command -v update-mime-database >/dev/null 2>&1; then
        update-mime-database "$HOME/.local/share/mime" >/dev/null 2>&1 \
            && ok "tipo MIME text/x-ssharp registrado." \
            || warn "no se pudo actualizar la base MIME."
    fi

    EDITOR_DESKTOP=""
    if [ -f "$HOME/.local/share/applications/code.desktop" ] || [ -f "/usr/share/applications/code.desktop" ]; then
        EDITOR_DESKTOP="code.desktop"
    elif [ -f "/usr/share/applications/org.kde.kate.desktop" ] || [ -f "$HOME/.local/share/applications/org.kde.kate.desktop" ]; then
        EDITOR_DESKTOP="org.kde.kate.desktop"
    elif [ -f "/usr/share/applications/org.gnome.gedit.desktop" ]; then
        EDITOR_DESKTOP="org.gnome.gedit.desktop"
    fi

    if [ -n "$EDITOR_DESKTOP" ] && command -v xdg-mime >/dev/null 2>&1; then
        if xdg-mime default "$EDITOR_DESKTOP" text/x-ssharp 2>/dev/null; then
            ok "archivos .ssharp asociados a $EDITOR_DESKTOP."
        else
            warn "no se pudo asociar el editor $EDITOR_DESKTOP."
        fi
    else
        warn "no se encontró Kate, VS Code ni gedit; se omite el editor por defecto."
    fi
fi

# --- 7. Optional: .desktop entry (app menu) ----------------------------------
step "Acceso directo en el menú de aplicaciones (opcional)..."
if [ -f "ssharp.desktop" ]; then
    APPS_DIR="$HOME/.local/share/applications"
    mkdir -p "$APPS_DIR" || warn "no se pudo crear $APPS_DIR."
    if cp -f "ssharp.desktop" "$APPS_DIR/ssharp.desktop"; then
        command -v update-desktop-database >/dev/null 2>&1 \
            && update-desktop-database "$APPS_DIR" >/dev/null 2>&1
        ok "'S# REPL' disponible en el menú de aplicaciones."
    else
        warn "no se pudo instalar ssharp.desktop."
    fi
else
    warn "ssharp.desktop no encontrado en el repo, se omite."
fi

printf '\nInstalación completa. Probá con: ssharp examples/hola.ssharp\n'
exit 0
