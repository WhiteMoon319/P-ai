#!/usr/bin/env bash
# 验证 patch 后的 ELF：检查 DT_STRTAB/DT_STRSZ 一致性、NEEDED 是否有效
cd "$(dirname "${BASH_SOURCE[0]}")/.."
python - <<'PY'
import struct, os

base = "src-tauri/gen/android/app/src/main/jniLibs/arm64-v8a"
for name in ["libproot_exec.so", "libproot_loader.so", "libtalloc.so", "libandroid-shmem.so"]:
    path = os.path.join(base, name)
    with open(path, "rb") as f:
        data = f.read()
    if data[:4] != b"\x7fELF":
        print(f"{name}: not ELF"); continue
    is64 = data[4] == 2
    endian = "<" if data[5] == 1 else ">"
    e_shoff = struct.unpack_from(endian + "Q", data, 40)[0] if is64 else struct.unpack_from(endian + "I", data, 32)[0]
    e_shentsize = struct.unpack_from(endian + "H", data, 58)[0] if is64 else struct.unpack_from(endian + "H", data, 46)[0]
    e_shnum = struct.unpack_from(endian + "H", data, 60)[0] if is64 else struct.unpack_from(endian + "H", data, 48)[0]
    e_shstrndx = struct.unpack_from(endian + "H", data, 62)[0] if is64 else struct.unpack_from(endian + "H", data, 50)[0]

    def cstr(off):
        try:
            end = data.index(b"\x00", off)
            return data[off:end].decode("utf-8", "replace")
        except ValueError:
            return "<OOB>"

    shstr_off = e_shoff + e_shstrndx * e_shentsize
    shstr_off = struct.unpack_from(endian + "Q", data, shstr_off + 24)[0] if is64 else struct.unpack_from(endian + "I", data, shstr_off + 16)[0]
    def sh_name(off):
        return cstr(shstr_off + struct.unpack_from(endian + "I", data, off)[0])

    strtab = strsz = None
    needed = []
    for i in range(e_shnum):
        sh_off = e_shoff + i * e_shentsize
        if sh_name(sh_off) != ".dynamic":
            continue
        if is64:
            dyn_off = struct.unpack_from(endian + "Q", data, sh_off + 24)[0]
            dyn_size = struct.unpack_from(endian + "Q", data, sh_off + 32)[0]
            es = 16
        else:
            dyn_off = struct.unpack_from(endian + "I", data, sh_off + 16)[0]
            dyn_size = struct.unpack_from(endian + "I", data, sh_off + 20)[0]
            es = 8
        for j in range(dyn_size // es):
            ent = dyn_off + j * es
            if is64:
                tag = struct.unpack_from(endian + "q", data, ent)[0]
                val = struct.unpack_from(endian + "Q", data, ent + 8)[0]
            else:
                tag = struct.unpack_from(endian + "i", data, ent)[0]
                val = struct.unpack_from(endian + "I", data, ent + 4)[0]
            if tag == 1:
                needed.append(val)
            elif tag == 5:
                strtab = val
            elif tag == 10:
                strsz = val
            elif tag == 29:
                print(f"  DT_RUNPATH={cstr(strtab + val) if strtab is not None and val < strsz else '<OOB>'}")
            elif tag == 0:
                break
    print(f"===== {name} =====")
    print(f"  DT_STRTAB={strtab} DT_STRSZ={strsz} file_size={len(data)}")
    if strtab is None or strsz is None:
        print("  !! 无 DT_STRTAB/DT_STRSZ")
    else:
        oob = strtab + strsz > len(data)
        print(f"  strtab+strsz={strtab + strsz} {'!! 越界' if oob else 'OK'}")
    for n in needed:
        s = cstr(strtab + n) if strtab is not None and n < strsz else "<OOB>"
        print(f"  NEEDED: {s}")
PY
