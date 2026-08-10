#!/usr/bin/env python3
"""patchelf 等价实现：修补 Android ELF 的 DT_NEEDED / SONAME / RUNPATH。

策略：只做「等长或缩短」的原地替换——新字符串长度 <= 旧字符串时，
直接在原位置写入（尾部补 \0），不扩展任何段、不改变文件布局。
这是对 bionic linker 最安全的方式（不触碰 phdr / 段布局）。

仅支持 ELF64 little-endian（arm64-v8a）。
用法：
  patch_elf.py <so> needed <old> <new>   # new 长度必须 <= old
  patch_elf.py <so> soname <new>         # 等长或缩短
  patch_elf.py <so> rpath <new>          # 等长或缩短
"""

import struct
import sys

def cstr_bytes(data, off):
    end = data.index(b"\x00", off)
    return data[off:end]

class ELF64:
    def __init__(self, path):
        self.path = path
        with open(path, "rb") as f:
            self.data = bytearray(f.read())
        assert self.data[:4] == b"\x7fELF"
        assert self.data[4] == 2
        assert self.data[5] == 1
        self.e_shoff = struct.unpack_from("<Q", self.data, 40)[0]
        self.e_shentsize = struct.unpack_from("<H", self.data, 58)[0]
        self.e_shnum = struct.unpack_from("<H", self.data, 60)[0]
        self.e_shstrndx = struct.unpack_from("<H", self.data, 62)[0]

    def save(self):
        with open(self.path, "wb") as f:
            f.write(self.data)


def patch(path, needed_map=None, soname=None, rpath=None):
    el = ELF64(path)
    data = el.data

    # 用 program headers (PT_DYNAMIC) 定位 dynamic 段，不依赖 section header
    e_phoff = struct.unpack_from("<Q", data, 32)[0]
    e_phentsize = struct.unpack_from("<H", data, 54)[0]
    e_phnum = struct.unpack_from("<H", data, 56)[0]

    dyn_off = dyn_size = None
    for i in range(e_phnum):
        ph = e_phoff + i * e_phentsize
        p_type = struct.unpack_from("<I", data, ph)[0]
        if p_type == 2:  # PT_DYNAMIC
            dyn_off = struct.unpack_from("<Q", data, ph + 8)[0]   # p_offset
            dyn_size = struct.unpack_from("<Q", data, ph + 32)[0]  # p_filesz
            break
    if dyn_off is None:
        print(f"{path}: 无 PT_DYNAMIC，跳过")
        return

    entries = []
    for i in range(dyn_size // 16):
        off = dyn_off + i * 16
        tag = struct.unpack_from("<q", data, off)[0]
        val = struct.unpack_from("<Q", data, off + 8)[0]
        entries.append({"off": off, "tag": tag, "val": val})
        if tag == 0:
            break

    # DT_STRTAB / DT_STRSZ
    strtab = None
    strsz = None
    for e in entries:
        if e["tag"] == 5:
            strtab = e["val"]
        elif e["tag"] == 10:
            strsz = e["val"]
    if strtab is None or strsz is None:
        print(f"{path}: 无 DT_STRTAB/DT_STRSZ，跳过")
        return

    def read_str(off):
        if off < 0 or off >= strsz:
            return ""
        try:
            return cstr_bytes(data, strtab + off).decode("utf-8", "replace")
        except ValueError:
            return ""

    def replace_at(off, old, new):
        old_b = old.encode() + b"\x00"
        new_b = new.encode() + b"\x00"
        if len(new_b) > len(old_b):
            print(f"  !! 新串 {new}({len(new)-1}) 长于旧串 {old}({len(old)-1})，跳过（不支持变长）")
            return False
        pos = strtab + off
        if data[pos:pos + len(old_b)] != old_b:
            # 允许旧串不带 \0 匹配（有些表不保证）
            pass
        # 写入新串 + \0，剩余位置清零
        data[pos:pos + len(new_b)] = new_b
        for i in range(len(new_b), len(old_b)):
            data[pos + i] = 0
        print(f"  [原地] {old} -> {new}")
        return True

    if needed_map:
        for e in entries:
            if e["tag"] == 1:
                s = read_str(e["val"])
                if s in needed_map:
                    replace_at(e["val"], s, needed_map[s])

    if soname:
        for e in entries:
            if e["tag"] == 0x0e:
                s = read_str(e["val"])
                if s:
                    replace_at(e["val"], s, soname)

    if rpath:
        found = False
        for e in entries:
            if e["tag"] == 29:
                s = read_str(e["val"])
                if s:
                    replace_at(e["val"], s, rpath)
                found = True
        if not found:
            print("  (无 DT_RUNPATH，跳过 rpath；依赖解析靠 LD_LIBRARY_PATH)")

    el.save()
    print(f"已修补 {path}")


if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)
    path = sys.argv[1]
    action = sys.argv[2]
    if action == "needed":
        patch(path, needed_map={sys.argv[3]: sys.argv[4]})
    elif action == "soname":
        patch(path, soname=sys.argv[3])
    elif action == "rpath":
        patch(path, rpath=sys.argv[3])
    else:
        print("unknown action")
        sys.exit(1)
