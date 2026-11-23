#!/usr/bin/env python3
"""Lists every place in a Target binary that forms an address of the PPI's
register window (0x4001_F000 to 0x4001_FFFF): docs/pacemaker task 3.3,
requirement PM33.3 (the Kernel writes the PPI at Init and never afterwards;
no Partition's code names the window).

Usage: ppi_scan.py <elf> [<llvm-objdump>]

Two forms are found in the disassembly: a `movw`/`movt` pair of one register
whose halves give an address of the window, and a literal-pool word in the
window. Each is printed with the function it lies in. The scan does not see
an address computed from other values at run time.
"""
import re
import subprocess
import sys

elf = sys.argv[1]
objdump = sys.argv[2] if len(sys.argv) > 2 else "/Library/Developer/CommandLineTools/usr/bin/llvm-objdump"
text = subprocess.run([objdump, "-d", "--no-show-raw-insn", "-C", elf], capture_output=True, text=True).stdout

func = "?"
low = {}
found = []
for line in text.splitlines():
    m = re.match(r"^[0-9a-f]+ <(.+)>:$", line)
    if m:
        func, low = m.group(1), {}
        continue
    m = re.match(r"^\s*([0-9a-f]+):\s+(\S+)\s+(.*)$", line)
    if not m:
        continue
    addr, op, args = m.groups()
    mw = re.match(r"(r\d+|lr|ip|sp), #(0x[0-9a-f]+|\d+)", args)
    if op.startswith("movw") and mw:
        low[mw.group(1)] = int(mw.group(2), 0)
    elif op.startswith("movt") and mw:
        high = int(mw.group(2), 0)
        value = (high << 16) | low.get(mw.group(1), 0)
        if high == 0x4001 and 0x4001F000 <= value <= 0x4001FFFF:
            found.append((func, addr, f"movw/movt 0x{value:08x}"))
    elif op == ".word" or op == ".long":
        try:
            value = int(args.split()[0], 0)
        except ValueError:
            continue
        if 0x4001F000 <= value <= 0x4001FFFF:
            found.append((func, addr, f"literal 0x{value:08x}"))
for f, a, what in found:
    print(f"{a}: {what} in {f}")
print(f"{len(found)} reference(s) to the PPI window in {elf}")
