#!/usr/bin/env bash
# Builds a release into dist/: a .deb, an AppImage, a tar.gz of /usr with its PKGBUILD for Arch
# (and good for any distro), and SHA256SUMS. Linux only for now. Usage: scripts/release.sh v0.2.0
# Needs cargo-packager (cargo install cargo-packager --locked).
set -euo pipefail
cd "$(dirname "$0")/.."

tag="${1:?usage: scripts/release.sh vX.Y.Z}"
version="${tag#v}"
crate_version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
if [[ "$version" != "$crate_version" ]]; then
  echo "tag $tag doesn't match Cargo.toml's version $crate_version: bump it first" >&2
  exit 1
fi

rm -rf dist
cargo build --release --locked
# linuxdeploy (inside the AppImage step) ships an old strip that fails on newer distros' libraries
# (.relr.dyn sections); the binaries are already release builds, so skip it.
NO_STRIP=1 cargo packager --release

# cargo-packager's PKGBUILD copies all of $srcdir, where makepkg also links the source tarball:
# the package would put the .tar.gz in /. Copy only usr/.
sed -i 's#cp -r "${srcdir}"/\* "${pkgdir}"/#cp -r "${srcdir}"/usr "${pkgdir}"/#' dist/PKGBUILD
grep -q '"${srcdir}"/usr' dist/PKGBUILD || { echo "PKGBUILD's package() changed: check the fix above" >&2; exit 1; }

cd dist
find . -maxdepth 1 -type f ! -name SHA256SUMS -printf '%f\n' | sort | xargs sha256sum > SHA256SUMS
ls -lh
