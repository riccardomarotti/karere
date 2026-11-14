#!/usr/bin/env bash
# Run ONLY in an ephemeral archlinux:base-devel container.
# Produces a portable installation tree tarball, not a package built by makepkg.
set -Eeuo pipefail
: "$PKGVER" "$PKGREL"

src=/work/source
work=/work/build
out=/work/artifacts
stage="$work/stage"
builddir="$work/meson"

test -f "$src/Cargo.lock"
grep -Eq "^version = \"$PKGVER\"$" "$src/Cargo.toml"
grep -Fq "version: '$PKGVER'" "$src/meson.build"

pacman -Syu --noconfirm --needed \
  base-devel meson ninja cmake rust cargo pkgconf \
  gtk4 libadwaita glib2 gdk-pixbuf2 libepoxy \
  alsa-lib nss libdrm libx11 libxcomposite libxdamage \
  libxext libxfixes libxkbcommon libxrandr mesa \
  wayland pango graphene cairo at-spi2-core \
  gettext blueprint-compiler python-gobject \
  appstream appstream-glib desktop-file-utils \
  patchelf unzip zstd curl

mkdir -p "$work" "$out"
python3 /recipe/cef_manifest.py "$src" "$work/cef.zip" x86_64

unzip -q "$work/cef.zip" -d "$work/cef-unpacked"
cefdir=''
for dir in "$work"/cef-unpacked/cef_binary_*_linux64_minimal; do
  [[ -d "$dir" ]] || continue
  if [[ -n "$cefdir" ]]; then
    echo 'More than one CEF directory in verified archive' >&2
    exit 1
  fi
  cefdir="$dir"
done
[[ -n "$cefdir" && -f "$cefdir/Release/libcef.so" ]]

flat="$work/cef-flat"
mkdir -p "$flat"
cp -a "$cefdir/include" "$cefdir/libcef_dll" "$cefdir/cmake" "$cefdir/CMakeLists.txt" "$flat/"
cp -a "$cefdir/Release/." "$cefdir/Resources/." "$flat/"
printf '{"type":"minimal","name":"%s","sha1":"0000000000000000000000000000000000000000"}\n' \
  "$(basename "$cefdir").tar.bz2" > "$flat/archive.json"

export CEF_PATH="$flat"
export RUSTFLAGS="--remap-path-prefix=/work=/build"
export CARGO_BUILD_JOBS=2
export PYTHON=/usr/bin/python3
export PATH="/usr/bin:$PATH"

meson setup "$builddir" "$src" \
  --prefix=/usr --bindir=lib/karere/cef --buildtype=release -Dprofile=default
meson compile -C "$builddir"
DESTDIR="$stage" meson install -C "$builddir"

real_binary="$stage/usr/lib/karere/cef/karere"
[[ -x "$real_binary" ]]
cp -a "$cefdir/Release/." "$cefdir/Resources/." "$stage/usr/lib/karere/cef/"

# Cargo may have embedded the temporary CEF_PATH as an RPATH. Shipping it
# would reference the runner instead of the installed CEF beside the binary.
patchelf --set-rpath '$ORIGIN' "$real_binary"
[[ "$(patchelf --print-rpath "$real_binary")" == '$ORIGIN' ]]

# Native Karere looks up gettext translations relative to the working dir.
mkdir -p "$stage/usr/share/karere" "$stage/usr/bin"
if [[ -d "$stage/usr/share/locale" ]]; then
  mv "$stage/usr/share/locale" "$stage/usr/share/karere/locale"
fi
cat > "$stage/usr/bin/karere" <<'EOF'
#!/bin/sh
cd /usr/share/karere || exit 1
exec /usr/lib/karere/cef/karere "$@"
EOF
chmod 755 "$stage/usr/bin/karere"
install -Dm644 "$src/LICENSE" "$stage/usr/share/licenses/karere-bin/LICENSE"

test -f "$stage/usr/lib/karere/cef/libcef.so"
test -d "$stage/usr/lib/karere/cef/locales"
test -f "$stage/usr/share/glib-2.0/schemas/io.github.tobagin.karere.gschema.xml"
desktop-file-validate "$stage/usr/share/applications/"*.desktop
appstream-util validate-relax --nonet "$stage/usr/share/metainfo/"*.xml
if ldd "$real_binary" | grep -q 'not found'; then
  echo 'Missing native shared library in the Arch builder' >&2
  exit 1
fi

asset="karere-$PKGVER-$PKGREL-linux-x86_64.tar.zst"
tar -C "$stage" -cf - usr | zstd -T2 -6 -o "$out/$asset"
(cd "$out" && sha256sum "$asset" > "$asset.sha256")
(cd "$out" && sha256sum -c "$asset.sha256")
tar -tf "$out/$asset" | grep -qx 'usr/bin/karere'
echo "Built $out/$asset"
