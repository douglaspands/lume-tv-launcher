#!/usr/bin/env bash
# Builds the HEIC decoder of the phone page: libheif with libde265 (LGPL-3.0) compiled to WebAssembly
# with Emscripten, only what decodes a HEIC/HEIF photo into RGBA (rotation and mirroring applied).
#
#   scripts/build-heic-wasm.sh
#
# Writes crates/lume-core/assets/companion/heic/heic.js, heic.wasm and SOURCES.txt (versions, URLs,
# hashes and build options: the corresponding source of the binaries). The files are committed, like
# the font subsets; the APK build only copies them. The sources are downloaded at fixed versions and
# their SHA-256 checked; the compiler is a fixed emscripten/emsdk image, run with the first of docker
# and podman found ($LUME_CONTAINER, a command with a docker-compatible "run", always wins). Two runs
# give the same bytes.
set -euo pipefail

LIBHEIF_VERSION=1.23.7
LIBHEIF_URL="https://github.com/strukturag/libheif/releases/download/v${LIBHEIF_VERSION}/libheif-${LIBHEIF_VERSION}.tar.gz"
LIBHEIF_SHA256=f7483e818503ea6224c8863e4a56ac02c7089021a98472c811c12aa19faf2475
LIBDE265_VERSION=1.1.3
LIBDE265_URL="https://github.com/strukturag/libde265/releases/download/v${LIBDE265_VERSION}/libde265-${LIBDE265_VERSION}.tar.gz"
LIBDE265_SHA256=554228bd17788c99a7e63b37ab5634722190e6e2bf60c1dcb01cef328e133905
EMSDK_IMAGE=docker.io/emscripten/emsdk:6.0.9

root="$(cd "$(dirname "$0")/.." && pwd)"
out="$root/crates/lume-core/assets/companion/heic"
work="$root/target/heic-wasm"

if [[ -n "${LUME_CONTAINER:-}" ]]; then
  # shellcheck disable=SC2206
  runtime=($LUME_CONTAINER)
elif command -v docker >/dev/null 2>&1; then
  runtime=(docker)
elif command -v podman >/dev/null 2>&1; then
  runtime=(podman)
else
  echo "error: no container runtime (docker or podman); set LUME_CONTAINER" >&2
  exit 1
fi
echo "Using container runtime: ${runtime[*]}"

mkdir -p "$work/src" "$out"
fetch() { # url sha256
  local file="$work/src/$(basename "$1")"
  if [[ ! -f "$file" ]] || ! echo "$2  $file" | sha256sum -c --quiet - 2>/dev/null; then
    curl -fsSL -o "$file.part" "$1"
    mv "$file.part" "$file"
  fi
  echo "$2  $file" | sha256sum -c --quiet - || { echo "error: SHA-256 of $(basename "$1") differs" >&2; exit 1; }
}
fetch "$LIBDE265_URL" "$LIBDE265_SHA256"
fetch "$LIBHEIF_URL" "$LIBHEIF_SHA256"

# The glue: one function for the page.
cat > "$work/src/heic_glue.c" <<'EOF'
// Decodes the primary picture of a HEIC/HEIF file into RGBA, with the rotation and mirroring the
// file records (irot/imir) applied. Returns a buffer of [width u32][height u32][RGBA rows] that the
// caller frees with free(), or 0 when the file does not decode.
#include <emscripten/emscripten.h>
#include <libheif/heif.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

EMSCRIPTEN_KEEPALIVE uint8_t *lume_heic_decode(const uint8_t *data, size_t size) {
  static int ready = 0;
  if (!ready) {
    heif_init(NULL);
    ready = 1;
  }
  uint8_t *result = NULL;
  struct heif_context *context = heif_context_alloc();
  struct heif_image_handle *handle = NULL;
  struct heif_image *image = NULL;
  if (heif_context_read_from_memory_without_copy(context, data, size, NULL).code != heif_error_Ok) goto done;
  if (heif_context_get_primary_image_handle(context, &handle).code != heif_error_Ok) goto done;
  if (heif_decode_image(handle, &image, heif_colorspace_RGB, heif_chroma_interleaved_RGBA, NULL).code != heif_error_Ok) goto done;
  int width = heif_image_get_width(image, heif_channel_interleaved);
  int height = heif_image_get_height(image, heif_channel_interleaved);
  size_t stride = 0;
  const uint8_t *plane = heif_image_get_plane_readonly2(image, heif_channel_interleaved, &stride);
  if (!plane || width <= 0 || height <= 0) goto done;
  result = malloc(8 + (size_t)width * height * 4);
  if (!result) goto done;
  uint32_t size_w = (uint32_t)width, size_h = (uint32_t)height;
  memcpy(result, &size_w, 4);
  memcpy(result + 4, &size_h, 4);
  for (int y = 0; y < height; y++) memcpy(result + 8 + (size_t)y * width * 4, plane + (size_t)y * stride, (size_t)width * 4);
done:
  if (image) heif_image_release(image);
  if (handle) heif_image_handle_release(handle);
  heif_context_free(context);
  return result;
}
EOF

# The build inside the container, from fixed paths so the output does not depend on this machine.
cat > "$work/src/build.sh" <<EOF
set -euo pipefail
export SOURCE_DATE_EPOCH=0
cd /work
rm -rf build prefix dist && mkdir -p build prefix dist
tar xzf src/libde265-${LIBDE265_VERSION}.tar.gz -C build
tar xzf src/libheif-${LIBHEIF_VERSION}.tar.gz -C build
FLAGS="-O3 -ffile-prefix-map=/work=."
cd /work/build/libde265-${LIBDE265_VERSION}
emcmake cmake -S . -B out -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DENABLE_SDL=OFF \
  -DENABLE_DECODER=OFF -DENABLE_ENCODER=OFF -DDISABLE_SSE=ON -DCMAKE_INSTALL_PREFIX=/work/prefix \
  -DCMAKE_C_FLAGS="\$FLAGS" -DCMAKE_CXX_FLAGS="\$FLAGS" > /dev/null
cmake --build out -j"\$(nproc)" > /dev/null
cmake --install out > /dev/null
cd /work/build/libheif-${LIBHEIF_VERSION}
emcmake cmake -S . -B out -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DBUILD_TESTING=OFF \
  -DCMAKE_INSTALL_PREFIX=/work/prefix -DCMAKE_PREFIX_PATH=/work/prefix \
  -DWITH_LIBDE265=ON -DLIBDE265_INCLUDE_DIR=/work/prefix/include -DLIBDE265_LIBRARY=/work/prefix/lib/libde265.a \
  -DWITH_X265=OFF -DWITH_KVAZAAR=OFF -DWITH_UVG266=OFF -DWITH_VVDEC=OFF -DWITH_VVENC=OFF \
  -DWITH_AOM_DECODER=OFF -DWITH_AOM_ENCODER=OFF -DWITH_DAV1D=OFF -DWITH_RAV1E=OFF -DWITH_SvtEnc=OFF \
  -DWITH_JPEG_DECODER=OFF -DWITH_JPEG_ENCODER=OFF -DWITH_OpenJPEG_DECODER=OFF -DWITH_OpenJPEG_ENCODER=OFF \
  -DWITH_OPENH264_DECODER=OFF -DWITH_FFMPEG_DECODER=OFF -DWITH_OpenJPH_ENCODER=OFF -DWITH_OpenJPH_DECODER=OFF \
  -DWITH_UNCOMPRESSED_CODEC=OFF -DWITH_HEADER_COMPRESSION=OFF -DWITH_LIBSHARPYUV=OFF -DWITH_GDK_PIXBUF=OFF \
  -DWITH_EXAMPLES=OFF -DWITH_EXAMPLE_HEIF_VIEW=OFF -DENABLE_PLUGIN_LOADING=OFF -DENABLE_MULTITHREADING_SUPPORT=OFF \
  -DWITH_REDUCED_VISIBILITY=OFF -DWITH_FUZZERS=OFF \
  -DCMAKE_C_FLAGS="\$FLAGS" -DCMAKE_CXX_FLAGS="\$FLAGS" > /dev/null
# Without libde265 libheif still builds, with no HEVC decoder: stop instead.
if grep -q "LIBDE265_.*NOTFOUND" out/CMakeCache.txt; then echo "error: libheif did not find libde265" >&2; exit 1; fi
cmake --build out -j"\$(nproc)" > /dev/null
cmake --install out > /dev/null
cd /work
emcc -O3 -ffile-prefix-map=/work=. -I prefix/include -c src/heic_glue.c -o build/heic_glue.o
em++ -O3 build/heic_glue.o prefix/lib/libheif.a prefix/lib/libde265.a \
  -sMODULARIZE=1 -sEXPORT_NAME=LumeHeic -sENVIRONMENT=web -sFILESYSTEM=0 -sALLOW_MEMORY_GROWTH=1 \
  -sEXPORTED_FUNCTIONS=_lume_heic_decode,_malloc,_free -sEXPORTED_RUNTIME_METHODS=HEAPU8 \
  -lembind -sDISABLE_EXCEPTION_CATCHING=1 -o dist/heic.js
EOF

"${runtime[@]}" run --rm -v "$work:/work:Z" -w /work "$EMSDK_IMAGE" bash src/build.sh

cp "$work/dist/heic.js" "$out/heic.js"
cp "$work/dist/heic.wasm" "$out/heic.wasm"
cat > "$out/SOURCES.txt" <<EOF
HEIC decoder of the Lume phone page (heic.js, heic.wasm)

Built by scripts/build-heic-wasm.sh from:
  libheif ${LIBHEIF_VERSION}  (LGPL-3.0)  ${LIBHEIF_URL}
    SHA-256 ${LIBHEIF_SHA256}
  libde265 ${LIBDE265_VERSION}  (LGPL-3.0)  ${LIBDE265_URL}
    SHA-256 ${LIBDE265_SHA256}
Compiler: ${EMSDK_IMAGE}
Options: static libraries, Release, only the libde265 (HEVC) decoder in libheif, no plugins, no
threads, no SIMD; the glue function lume_heic_decode (in the script) returns the primary picture as
RGBA with the file's rotation and mirroring applied.

heic.js  SHA-256 $(sha256sum "$out/heic.js" | cut -d' ' -f1)
heic.wasm SHA-256 $(sha256sum "$out/heic.wasm" | cut -d' ' -f1)
EOF
ls -l "$out"
