#!/usr/bin/env bash
# Install Snipdeck's packages on a row of distributions, in containers.
#
#   packaging/linux/test-install.sh [dist/<version>] [image …]
#
# For each image: install the .deb, .rpm or tarball with that distribution's
# own tools and only the hard dependencies (no recommendations), then check
# that every library the binary links resolves, and start it with no display.
# "No display" is the expected end: the loader, glibc and every linked library
# got as far as asking for a window. What a desktop does with the app is not
# visible here — that takes a virtual machine.
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
version="$(grep -m1 '^version' "$here/Cargo.toml" | sed 's/.*"\(.*\)"/\1/')"
dist="$here/dist/$version"
if [ -d "${1:-}" ]; then dist="$1"; shift; fi
dist="$(realpath "$dist")"
[ -f "$dist/SHA256SUMS" ] || { echo "no packages in $dist — run package.sh" >&2; exit 1; }
images=("$@")
[ ${#images[@]} -gt 0 ] || images=(
    debian:12 debian:13 ubuntu:22.04 ubuntu:24.04
    fedora:latest rockylinux/rockylinux:9
    opensuse/leap:15.6 opensuse/tumbleweed:latest archlinux:latest
)

check='
  B=$(command -v snipdeck || echo /root/.local/bin/snipdeck)
  [ -x "$B" ] || { echo "binary: none installed"; exit 1; }
  echo "binary: $B"
  missing=$(ldd "$B" | grep "not found")
  [ -z "$missing" ] && echo "ldd: all resolved" || { echo "ldd MISSING:"; echo "$missing"; }
  out=$(env -u DISPLAY -u WAYLAND_DISPLAY timeout 15 "$B" 2>&1 | tail -3)
  echo "start: $out"
'
pass=0; fail=0
for img in "${images[@]}"; do
    case "$img" in
        debian:*|ubuntu:*)
            install='export DEBIAN_FRONTEND=noninteractive; apt-get update -qq >/dev/null &&
                     apt-get install -y -qq --no-install-recommends /p/*.deb >/dev/null' ;;
        fedora:*|*rockylinux*)
            install='dnf install -y -q --setopt=install_weak_deps=False /p/*.rpm >/dev/null' ;;
        *opensuse*)
            install='zypper -q --non-interactive install --no-recommends --allow-unsigned-rpm /p/*.rpm >/dev/null' ;;
        archlinux:*)
            # No Arch package (yet): the tarball and its installer, with the
            # four libraries the binary links installed by hand.
            install='pacman -Sy --noconfirm --needed gtk3 libxcb dbus fontconfig >/dev/null &&
                     tar -C /tmp -xzf /p/snipdeck-linux-x64.tar.gz &&
                     HOME=/root /tmp/snipdeck-linux-x64/install.sh >/dev/null' ;;
        *) echo "$img: no install recipe"; continue ;;
    esac
    ref=$img; case "$img" in opensuse/*) ref="registry.opensuse.org/$img" ;; esac
    echo "=== $img"
    result=$(podman run --rm -v "$dist":/p:ro,Z "$ref" sh -c "{ $install; } || exit 1; echo installed; $check" 2>&1 | tail -12)
    echo "$result" | sed 's/^/    /'
    if echo "$result" | grep -qx "installed" && echo "$result" | grep -q "ldd: all resolved" &&
   ! echo "$result" | grep -qi "error while loading\|GLIBC_.*not found\|binary: none"; then
        pass=$((pass + 1))
    else
        fail=$((fail + 1))
    fi
done
echo "--- $pass passed, $fail failed"
