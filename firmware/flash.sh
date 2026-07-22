#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

# Optional:
#   PORT=/dev/ttyACM0 ./flash.sh
#   MONITOR=1 ./flash.sh
PORT="${PORT:-}"
MONITOR="${MONITOR:-0}"

cargo build --release

BIN="target/riscv32imc-unknown-none-elf/release/oled-dash"
IMG="target/oled-dash-merged.bin"

# Build a merged image, then patch the app descriptor exactly where this board's
# bootloader expects it (factory app +0x40), and finally recompute image checks.
espflash save-image --chip esp32c3 --merge --ignore-app-descriptor "$BIN" "$IMG" >/dev/null

python3 - "$IMG" <<'PY'
import hashlib
import struct
import sys

path = sys.argv[1]
APP_BASE = 0x10000
APP_DESC_OFFSET = APP_BASE + 0x20 + 0x20

def cstr(text: str, n: int) -> bytes:
    b = text.encode("ascii", "ignore")[: n - 1]
    return b + b"\x00" * (n - len(b))

with open(path, "rb") as f:
    data = bytearray(f.read())

# Patch descriptor fields.
desc = bytearray(256)
struct.pack_into("<I", desc, 0, 0xABCD5432)      # magic_word
struct.pack_into("<I", desc, 4, 0)               # secure_version
desc[16:48] = cstr("0.1.0", 32)                  # version
desc[48:80] = cstr("oled-dash", 32)              # project_name
desc[80:96] = cstr("00:00:00", 16)               # build time
desc[96:112] = cstr("1970-01-01", 16)            # build date
desc[112:144] = cstr("esp-hal", 32)              # idf_ver
struct.pack_into("<H", desc, 176, 0)             # min_efuse_blk_rev_full
struct.pack_into("<H", desc, 178, 0xFFFF)        # max_efuse_blk_rev_full
desc[180] = 0                                    # mmu_page_size(log2)
data[APP_DESC_OFFSET:APP_DESC_OFFSET + 256] = desc

# Recompute ESP image checksum for app partition at 0x10000.
base = APP_BASE
segment_count = data[base + 1]
append_digest = data[base + 23]
pos = base + 24
checksum = 0xEF

for _ in range(segment_count):
    seg_len = struct.unpack_from("<I", data, pos + 4)[0]
    pos += 8
    for b in data[pos:pos + seg_len]:
        checksum ^= b
    pos += seg_len

pad = (15 - ((pos - base) % 16)) % 16
pos += pad
data[pos] = checksum
end = pos + 1

# Recompute optional SHA-256 digest if present.
if append_digest:
    digest = hashlib.sha256(data[base:end]).digest()
    data[end:end + 32] = digest

with open(path, "wb") as f:
    f.write(data)
PY

ARGS=(write-bin --chip esp32c3 0x0 "$IMG")

if [[ -n "$PORT" ]]; then
  ARGS+=(--port "$PORT")
fi

if [[ "$MONITOR" == "1" ]]; then
  ARGS+=(--monitor)
fi

exec espflash "${ARGS[@]}"
