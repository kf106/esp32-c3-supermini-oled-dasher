#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

# Cursor shell sessions may set CARGO_TARGET_DIR to a shared cache; that makes
# cargo write elsewhere while we flash a stale ELF from local target/.
unset CARGO_TARGET_DIR

# Optional:
#   PORT=/dev/ttyACM0 ./flash.sh
#   MONITOR=1 ./flash.sh
#   ./flash.sh 7   # seed progress to level 7 (SAVE_PAGE slot 0)
PORT="${PORT:-}"
MONITOR="${MONITOR:-0}"

SEED_LEVEL=""
if [[ $# -ge 1 ]]; then
  if ! [[ "$1" =~ ^[0-9]+$ ]] || (( $1 < 1 || $1 > 26 )); then
    echo "usage: ./flash.sh [level]" >&2
    echo "  level: 1..26 — seed save progress for that level in the flashed image" >&2
    exit 1
  fi
  SEED_LEVEL="$1"
fi

BIN="target/riscv32imc-unknown-none-elf/release/oled-dash"
IMG="target/oled-dash-merged.bin"
OFFSET_RS="src/save_flash_offset.rs"

build_and_merge() {
  cargo build --release
  espflash save-image --chip esp32c3 --merge --ignore-app-descriptor "$BIN" "$IMG" >/dev/null
}

sync_save_offset() {
  python3 - "$IMG" "$BIN" "$OFFSET_RS" <<'PY'
import struct, subprocess, sys, os
img_path, elf_path, offset_rs = sys.argv[1], sys.argv[2], sys.argv[3]
APP_BASE = 0x10000

def save_page_vaddr(elf_path: str) -> int:
    out = subprocess.check_output(["nm", elf_path], text=True)
    for line in out.splitlines():
        parts = line.split()
        if len(parts) >= 3 and parts[-1] == "SAVE_PAGE":
            return int(parts[0], 16)
    raise SystemExit("SAVE_PAGE symbol not found in ELF (nm)")

def save_page_flash_offset(img: bytes, vaddr: int) -> int:
    if img[APP_BASE] != 0xE9:
        raise SystemExit(f"bad ESP image magic at 0x{APP_BASE:X}")
    segments = img[APP_BASE + 1]
    pos = APP_BASE + 24
    for _ in range(segments):
        load_addr, size = struct.unpack_from("<II", img, pos)
        data_off = pos + 8
        if load_addr <= vaddr < load_addr + size:
            off = data_off + (vaddr - load_addr)
            if off % 4096 != 0:
                raise SystemExit(f"SAVE_PAGE offset 0x{off:X} not sector-aligned")
            return off
        pos = data_off + size
    raise SystemExit(f"SAVE_PAGE vaddr 0x{vaddr:X} not in ESP image segments")

img = open(img_path, "rb").read()
vaddr = save_page_vaddr(elf_path)
off = save_page_flash_offset(img, vaddr)
text = (
    "// Auto-updated by flash.sh from the linked ELF + merged image. Do not edit.\n"
    f"pub const SAVE_FLASH_OFFSET: u32 = 0x{off:X};\n"
)
old = open(offset_rs).read() if os.path.exists(offset_rs) else ""
if old != text:
    open(offset_rs, "w").write(text)
    print(f"updated {offset_rs}: SAVE_FLASH_OFFSET=0x{off:X}")
    sys.exit(2)
print(f"SAVE_FLASH_OFFSET=0x{off:X}")
print(off)
PY
}

build_and_merge
set +e
sync_out=$(sync_save_offset)
sync_rc=$?
set -e
echo "$sync_out"
if [[ "$sync_rc" -eq 2 ]]; then
  build_and_merge
  set +e
  sync_out=$(sync_save_offset)
  sync_rc=$?
  set -e
  echo "$sync_out"
  if [[ "$sync_rc" -eq 2 ]]; then
    echo "error: SAVE_FLASH_OFFSET kept changing after rebuild" >&2
    exit 1
  fi
elif [[ "$sync_rc" -ne 0 ]]; then
  exit "$sync_rc"
fi

SAVE_OFF=$(echo "$sync_out" | tail -n1)

python3 - "$IMG" "$SAVE_OFF" "${SEED_LEVEL}" <<'PY'
import struct, sys
path, save_off_s, seed = sys.argv[1], sys.argv[2], sys.argv[3]
save_off = int(save_off_s, 0)
APP_BASE = 0x10000
APP_DESC_OFFSET = APP_BASE + 0x20 + 0x20

def cstr(text: str, n: int) -> bytes:
    b = text.encode("ascii", "ignore")[: n - 1]
    return b + b"\x00" * (n - len(b))

with open(path, "rb") as f:
    data = bytearray(f.read())

desc = bytearray(256)
struct.pack_into("<I", desc, 0, 0xABCD5432)
struct.pack_into("<I", desc, 4, 0)
desc[16:48] = cstr("0.1.0", 32)
desc[48:80] = cstr("oled-dash", 32)
desc[80:96] = cstr("00:00:00", 16)
desc[96:112] = cstr("1970-01-01", 16)
desc[112:144] = cstr("esp-hal", 32)
struct.pack_into("<H", desc, 176, 0)
struct.pack_into("<H", desc, 178, 0xFFFF)
desc[180] = 0
data[APP_DESC_OFFSET:APP_DESC_OFFSET + 256] = desc

# Clear the whole SAVE_PAGE to 0xFF, then seed slot 0 if requested.
if len(data) >= save_off + 4096:
    for i in range(4096):
        data[save_off + i] = 0xFF

if seed:
    level = int(seed) - 1
    magic = 0x4853_4144
    version = 1
    cksum = magic ^ version ^ level ^ 0xA5A5_C3C3
    data[save_off : save_off + 16] = struct.pack("<IIII", magic, version, level, cksum)
    print(f"seeding progress: level {seed} (index {level}) @ 0x{save_off:X}")

# Runtime save appends into SAVE_PAGE (inside the app image). That would break a
# trailing SHA-256, so disable hash_appended — bootloader then skips the check.
data[APP_BASE + 23] = 0

base = APP_BASE
segment_count = data[base + 1]
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

open(path, "wb").write(data)
PY

ARGS=(write-bin --chip esp32c3 0x0 "$IMG")
if [[ -n "$PORT" ]]; then ARGS+=(--port "$PORT"); fi
if [[ "$MONITOR" == "1" ]]; then ARGS+=(--monitor); fi
exec espflash "${ARGS[@]}"
