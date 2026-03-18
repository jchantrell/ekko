#!/bin/sh
set -e

REPO="jchantrell/ekko"
BINARY="ekko"
INSTALL_DIR="${EKKO_INSTALL_DIR:-/usr/local/bin}"

main() {
    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    arch=$(uname -m)

    case "$os" in
        linux)  target_os="unknown-linux-gnu" ;;
        darwin) target_os="apple-darwin" ;;
        *)      echo "Unsupported OS: $os" >&2; exit 1 ;;
    esac

    case "$arch" in
        x86_64|amd64)  target_arch="x86_64" ;;
        aarch64|arm64) target_arch="aarch64" ;;
        *)             echo "Unsupported architecture: $arch" >&2; exit 1 ;;
    esac

    target="${target_arch}-${target_os}"

    version=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' | head -1 | cut -d'"' -f4)

    if [ -z "$version" ]; then
        echo "Failed to fetch latest version" >&2
        exit 1
    fi

    archive="${BINARY}-${version}-${target}.tar.gz"
    url="https://github.com/${REPO}/releases/download/${version}/${archive}"

    echo "Downloading ${BINARY} ${version} for ${target}..."

    tmpdir=$(mktemp -d)
    trap 'rm -rf "$tmpdir"' EXIT

    curl -fsSL "$url" -o "${tmpdir}/${archive}"
    tar xzf "${tmpdir}/${archive}" -C "$tmpdir"

    if [ -w "$INSTALL_DIR" ]; then
        cp "${tmpdir}/${BINARY}-${version}-${target}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
    else
        echo "Installing to ${INSTALL_DIR} (requires sudo)..."
        sudo cp "${tmpdir}/${BINARY}-${version}-${target}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
    fi

    chmod +x "${INSTALL_DIR}/${BINARY}"
    echo "Installed ${BINARY} ${version} to ${INSTALL_DIR}/${BINARY}"
}

main
