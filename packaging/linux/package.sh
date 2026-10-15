#!/usr/bin/env bash
# Assemble Snipdeck's Linux release from a binary that is already built.
#
#   packaging/linux/package.sh            → dist/<version>/
#       snipdeck-linux-x64.tar.gz          the tarball install.sh installs from
#       snipdeck_<version>-1_amd64.deb     Debian, Ubuntu
#       snipdeck-<version>-1.x86_64.rpm    Fedora, openSUSE
#       SHA256SUMS
#
# Nothing here compiles. The binary is SNIPDECK_BIN, else the one built in
# the Debian 11 image (target/container/release — glibc 2.31, the one to
# ship; see build-release.sh), else target/release. Whichever is used is
# printed.
#
# Needs cargo-deb and cargo-generate-rpm (`cargo install` both).
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$here"

version="$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)"/\1/')"
if [ -n "${SNIPDECK_BIN:-}" ]; then
    bin=$SNIPDECK_BIN
elif [ -x target/container/release/snipdeck ]; then
    bin=target/container/release/snipdeck
else
    bin=target/release/snipdeck
fi
[ -x "$bin" ] || { echo "no binary at $bin — build first" >&2; exit 1; }
for tool in cargo-deb cargo-generate-rpm; do
    command -v "$tool" >/dev/null || { echo "$tool is not installed: cargo install $tool" >&2; exit 1; }
done
echo "binary:   $bin  (version $version)"
echo "glibc:    needs $(objdump -T "$bin" | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1)"

ext=snipdeck@revisetouch.com
out=dist/$version
rm -rf "$out"
mkdir -p "$out"

# One staging directory: the asset lists in Cargo.toml name target/package and
# nothing else, so which binary was chosen stays out of the metadata.
stage=target/package
rm -rf "$stage"
install -d "$stage/icons" "$stage/$ext"
install -m755 "$bin" "$stage/snipdeck"
install -m644 packaging/gnome-extension/$ext/extension.js packaging/gnome-extension/$ext/metadata.json "$stage/$ext/"
# A package puts the binary on PATH; the tarball's install.sh rewrites Exec
# for ~/.local/bin itself.
sed 's|^Exec=.*|Exec=snipdeck|' packaging/linux/snipdeck.desktop > "$stage/snipdeck.desktop"
for size in 48 128 256; do
    magick assets/preview_256.png -resize "${size}x${size}" "$stage/icons/snipdeck-$size.png"
done

# The tarball: the layout install.sh expects beside it.
tar_dir=target/package-tar/snipdeck-linux-x64
rm -rf target/package-tar && mkdir -p "$tar_dir"
install -m755 "$bin" "$tar_dir/snipdeck"
install -m755 packaging/linux/install.sh "$tar_dir/"
install -m644 packaging/linux/snipdeck.desktop "$tar_dir/"
install -m644 assets/preview_256.png "$tar_dir/snipdeck.png"
cp -r packaging/gnome-extension/$ext "$tar_dir/"
tar -C target/package-tar -czf "$out/snipdeck-linux-x64.tar.gz" snipdeck-linux-x64

cargo deb --no-build --no-strip --output "$out/" >/dev/null
cargo generate-rpm --output "$out/" >/dev/null

(cd "$out" && sha256sum -- * > SHA256SUMS)
ls -l "$out"
