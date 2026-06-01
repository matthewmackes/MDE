#!/usr/bin/env bash
# install-helpers/build-lizardfs.sh — MESHFS-1.1
#
# Build LizardFS from a pinned tag and produce lizardfs-binaries.tar.gz
# suitable for use as Source1 in the MDE RPM.
#
# Usage:
#   install-helpers/build-lizardfs.sh [tag]
#   install-helpers/build-lizardfs.sh 3.13.0-rc2
#
# Output:
#   lizardfs-binaries.tar.gz  (in the current directory)
#
# This script mirrors the build steps in .github/workflows/lizardfs-build.yml
# for local dev use. The operator runs this once before `make rpm` so
# rpmbuild/SOURCES/lizardfs-binaries.tar.gz is in place.

set -euo pipefail

LIZARDFS_TAG="${1:-3.13.0-rc2}"
LIZARDFS_REPO="https://github.com/lizardfs/lizardfs.git"
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

echo "build-lizardfs.sh: cloning tag ${LIZARDFS_TAG} ..."
git clone --depth 1 --branch "${LIZARDFS_TAG}" "${LIZARDFS_REPO}" "${WORK}/src"

echo "build-lizardfs.sh: configuring ..."
mkdir -p "${WORK}/build"
cmake "${WORK}/src" \
    -B "${WORK}/build" \
    -DCMAKE_POLICY_VERSION_MINIMUM=3.5 \
    -DCMAKE_CXX_FLAGS="-include cstdint" \
    -DCMAKE_CXX_STANDARD_LIBRARIES="-lfmt" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX=/usr \
    -DENABLE_TESTS=OFF \
    -DENABLE_WERROR=OFF

echo "build-lizardfs.sh: building ..."
cmake --build "${WORK}/build" --parallel "$(nproc)" -- \
    mfsmaster mfschunkserver mfsmount mfsmetarestore mfscli \
    mfssetgoal mfssetquota 2>/dev/null \
    || cmake --build "${WORK}/build" --parallel "$(nproc)"

echo "build-lizardfs.sh: bundling binaries ..."
BIN_DIR="${WORK}/bins"
mkdir -p "${BIN_DIR}"
for b in mfsmaster mfschunkserver mfsmount mfsmetarestore mfscli mfssetgoal mfssetquota; do
    src=$(find "${WORK}/build" -name "${b}" -type f | head -1)
    if [ -n "${src}" ]; then
        cp "${src}" "${BIN_DIR}/${b}"
        strip "${BIN_DIR}/${b}" 2>/dev/null || true
    fi
done
ls -lh "${BIN_DIR}/"

OUT="lizardfs-binaries.tar.gz"
tar -czf "${OUT}" -C "${BIN_DIR}" .
echo "build-lizardfs.sh: wrote ${OUT} ($(du -sh "${OUT}" | cut -f1))"
echo "build-lizardfs.sh: copy ${OUT} to rpmbuild/SOURCES/ before make rpm"
