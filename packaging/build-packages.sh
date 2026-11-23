#!/usr/bin/env bash
# Build Shift Linux packages: RPM, DEB and AppImage into dist/.
#
#   packaging/build-packages.sh            # all formats
#   packaging/build-packages.sh rpm deb    # selected formats
#
# Needs: cargo, rpmbuild (rpm), dpkg-deb + fakeroot (deb), appimagetool
# (AppImage; path in $APPIMAGETOOL or on PATH).
#
# The packages use the system's GTK 4 (>= 4.12) and OpenSSL 3; they are not
# bundled. The binary needs glibc >= the one it was built against (check
# with: objdump -T target/release/shift | grep -o 'GLIBC_[0-9.]*' | sort -V | tail -1).
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT=$PWD
APP_ID=com.ensignsoft.Shift
NAME=shift
BIN=shift-client                     # "shift" is a shell builtin
VERSION=$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -1)
ARCH_RPM=x86_64
ARCH_DEB=amd64
DIST=$ROOT/dist
WORK=${TMPDIR:-/tmp}/shift-packaging   # rpmbuild cannot handle spaces in paths
FORMATS=("$@")
[ ${#FORMATS[@]} -eq 0 ] && FORMATS=(rpm deb appimage)

want() { local f; for f in "${FORMATS[@]}"; do [ "$f" = "$1" ] && return 0; done; return 1; }

echo ">> cargo build --release"
cargo build --release
rm -rf "$WORK" && mkdir -p "$WORK" "$DIST"

# ---------------------------------------------------------------- staging
STAGE=$WORK/stage
install -Dm755 target/release/shift "$STAGE/usr/bin/$BIN"
strip --strip-unneeded "$STAGE/usr/bin/$BIN"
install -Dm644 "data/$APP_ID.desktop" "$STAGE/usr/share/applications/$APP_ID.desktop"
install -Dm644 "packaging/linux/$APP_ID.metainfo.xml" "$STAGE/usr/share/metainfo/$APP_ID.metainfo.xml"
for s in 16 24 32 48 64 128 256 512; do
  install -Dm644 "data/icons/hicolor/${s}x${s}/apps/$APP_ID.png" \
    "$STAGE/usr/share/icons/hicolor/${s}x${s}/apps/$APP_ID.png"
done
install -Dm644 README.md "$STAGE/usr/share/doc/$NAME/README.md"
install -Dm644 data/icons/shift/LICENSE-phosphor "$STAGE/usr/share/doc/$NAME/LICENSE-phosphor"

SUMMARY="FTP, FTPS ve SFTP dosya aktarım istemcisi"
MAINTAINER=${MAINTAINER:-"$(git config user.name) <$(git config user.email)>"}

# -------------------------------------------------------------------- RPM
if want rpm; then
  echo ">> RPM"
  TOP=$WORK/rpmbuild
  mkdir -p "$TOP"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
  cat > "$TOP/SPECS/$NAME.spec" <<EOF
Name:           $NAME
Version:        $VERSION
Release:        1%{?dist}
Summary:        $SUMMARY
License:        LicenseRef-NotSpecified
URL:            https://ensignsoft.com
Requires:       gtk4 >= 4.12
Requires:       hicolor-icon-theme

%description
Shift is a modern, lightweight file transfer client for FTP, FTPS and SFTP.
Command: $BIN

%install
mkdir -p "%{buildroot}" && cp -a "$STAGE/." "%{buildroot}/"

%files
/usr/bin/$BIN
/usr/share/applications/$APP_ID.desktop
/usr/share/metainfo/$APP_ID.metainfo.xml
/usr/share/icons/hicolor/*/apps/$APP_ID.png
%doc /usr/share/doc/$NAME/README.md
%license /usr/share/doc/$NAME/LICENSE-phosphor

%changelog
* $(LC_ALL=C date '+%a %b %d %Y') $MAINTAINER - $VERSION-1
- Package build
EOF
  rpmbuild -bb --quiet --define "_topdir $TOP" --define "debug_package %{nil}" \
    --define "__strip /bin/true" "$TOP/SPECS/$NAME.spec"
  find "$TOP/RPMS" -name '*.rpm' -exec cp {} "$DIST/" \;
fi

# -------------------------------------------------------------------- DEB
if want deb; then
  echo ">> DEB"
  DEB=$WORK/deb
  cp -a "$STAGE" "$DEB"
  mkdir -p "$DEB/DEBIAN"
  SIZE=$(du -sk "$DEB/usr" | cut -f1)
  cat > "$DEB/DEBIAN/control" <<EOF
Package: $NAME
Version: $VERSION
Section: net
Priority: optional
Architecture: $ARCH_DEB
Installed-Size: $SIZE
Depends: libc6 (>= 2.38), libgtk-4-1 (>= 4.12), libssl3t64 | libssl3, hicolor-icon-theme
Maintainer: $MAINTAINER
Homepage: https://ensignsoft.com
Description: $SUMMARY
 Shift is a modern, lightweight file transfer client for FTP, FTPS and SFTP.
 Command: $BIN
EOF
  find "$DEB" -type d -exec chmod 755 {} +
  fakeroot dpkg-deb --build --root-owner-group "$DEB" "$DIST/${NAME}_${VERSION}_${ARCH_DEB}.deb" >/dev/null
fi

# --------------------------------------------------------------- AppImage
if want appimage; then
  echo ">> AppImage"
  TOOL=${APPIMAGETOOL:-$(command -v appimagetool || true)}
  if [ -z "$TOOL" ] || [ ! -x "$TOOL" ]; then
    echo "appimagetool bulunamadı (APPIMAGETOOL=/yol/appimagetool-x86_64.AppImage)" >&2
    exit 1
  fi
  APPDIR=$WORK/Shift.AppDir
  cp -a "$STAGE" "$APPDIR"
  cp "data/$APP_ID.desktop" "$APPDIR/$APP_ID.desktop"
  cp "data/icons/hicolor/256x256/apps/$APP_ID.png" "$APPDIR/$APP_ID.png"
  cp "$APPDIR/$APP_ID.png" "$APPDIR/.DirIcon"
  cat > "$APPDIR/AppRun" <<'RUN'
#!/bin/sh
HERE=$(dirname "$(readlink -f "$0")")
exec "$HERE/usr/bin/shift-client" "$@"
RUN
  chmod +x "$APPDIR/AppRun"
  ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$TOOL" --no-appstream "$APPDIR" \
    "$DIST/Shift-${VERSION}-x86_64.AppImage" >/dev/null
fi

echo ">> dist/"
ls -lh "$DIST"
