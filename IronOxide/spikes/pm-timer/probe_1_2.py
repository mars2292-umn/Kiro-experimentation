#!/usr/bin/env python3
"""Task 1.2 of docs/pacemaker/tasks.md: one probe per register behaviour the
design relies on.

Board A runs the spike built with the feature `idle` (it reopens the debug
port, starts the crystal oscillator and spins). Every register is then read
and written through the debug probes with probe-rs; the peripherals run
without the core. Board B is used only to drive the sense line P1.01 and to
read P1.03.

Usage: probe_1_2.py <probe A> <probe B> <record.json>

Each probe records the reads that decide it and a verdict: "confirmed" or
"refuted". The script exits 0 when every behaviour is confirmed.
"""
import json
import subprocess
import sys
import time

CHIP = "nRF52840_xxAA"
A, B, RECORD = sys.argv[1], sys.argv[2], sys.argv[3]

GPIOTE = 0x4000_6000
TIMER1 = 0x4000_9000
TIMER3 = 0x4001_A000
PPI = 0x4001_F000
P1 = 0x5000_0300
NVIC_ISPR0 = 0xE000_E200
NVIC_ICPR0 = 0xE000_E280
NVIC_ISER0 = 0xE000_E100

# TIMER
T_START, T_STOP, T_COUNT, T_CLEAR = 0x000, 0x004, 0x008, 0x00C
T_INTENSET, T_INTENCLR = 0x304, 0x308
T_MODE, T_BITMODE, T_PRESCALER = 0x504, 0x508, 0x510


def t_capture(n): return 0x040 + 4 * n
def t_event(n): return 0x140 + 4 * n
def t_cc(n): return 0x540 + 4 * n


# GPIOTE
def g_set(n): return GPIOTE + 0x030 + 4 * n
def g_clr(n): return GPIOTE + 0x060 + 4 * n
def g_in(n): return GPIOTE + 0x100 + 4 * n
def g_config(n): return GPIOTE + 0x510 + 4 * n


# PPI
PPI_CHENSET, PPI_CHENCLR = PPI + 0x504, PPI + 0x508
def ppi_eep(n): return PPI + 0x510 + 8 * n
def ppi_tep(n): return PPI + 0x514 + 8 * n
def ppi_fork(n): return PPI + 0x910 + 4 * n


# Port 1
P_OUTSET, P_OUTCLR, P_IN, P_DIR, P_DIRSET, P_DIRCLR = 0x508, 0x50C, 0x510, 0x514, 0x518, 0x51C
def pin_cnf(n): return P1 + 0x700 + 4 * n


def probe(board, *args):
    out = subprocess.run(["probe-rs", *args[:1], "b32", "--chip", CHIP, "--probe", board, *args[1:]],
                         capture_output=True, text=True)
    if out.returncode != 0:
        sys.exit(f"probe-rs {' '.join(args)} on {board}: {out.stderr.strip()}")
    return out.stdout


def rd(board, addr):
    return int(probe(board, "read", f"0x{addr:08x}", "1").split()[-1], 16)


def wr(board, addr, value):
    probe(board, "write", f"0x{addr:08x}", f"0x{value & 0xFFFFFFFF:08x}")


def a_rd(addr): return rd(A, addr)
def a_wr(addr, value): wr(A, addr, value)


def now3(through=2):
    """TIMER3's count, through a capture register (which the capture
    overwrites: not the compare under test)."""
    a_wr(TIMER3 + t_capture(through), 1)
    return a_rd(TIMER3 + t_cc(through))


def reset_peripherals():
    a_wr(PPI_CHENCLR, 0xFFFFFFFF)
    for t in (TIMER1, TIMER3):
        a_wr(t + T_STOP, 1)
        a_wr(t + T_CLEAR, 1)
        a_wr(t + T_INTENCLR, 0xFFFFFFFF)
        a_wr(t + T_MODE, 0)
        a_wr(t + T_BITMODE, 3)
        a_wr(t + T_PRESCALER, 4)
        for n in range(6):
            a_wr(t + t_event(n), 0)
            a_wr(t + t_cc(n), 0)
    for n in range(8):
        a_wr(g_config(n), 0)
        a_wr(g_in(n), 0)
    for n in range(4):
        a_wr(ppi_eep(n), 0)
        a_wr(ppi_tep(n), 0)
        a_wr(ppi_fork(n), 0)
    a_wr(NVIC_ICPR0, 0xFFFFFFFF)


results = []


def record(key, behaviour, reads, confirmed, note=""):
    results.append({"probe": key, "behaviour": behaviour, "verdict": "confirmed" if confirmed else "refuted",
                    "reads": reads, "note": note})
    print(f"{key}: {'confirmed' if confirmed else 'REFUTED'}", flush=True)


def hexd(d):
    return {k: (f"0x{v:08x}" if isinstance(v, int) and not isinstance(v, bool) else v) for k, v in d.items()}


# --------------------------------------------------------------------------
reset_peripherals()
enabled = a_rd(NVIC_ISER0)

# P1: TIMER3 has six compare registers and one interrupt line (26).
reads = {}
for n in range(6):
    a_wr(TIMER3 + t_cc(n), 0x1000_0000 + 0x111 * (n + 1))
cc3 = [a_rd(TIMER3 + t_cc(n)) for n in range(6)]
for n in range(6):
    a_wr(TIMER1 + t_cc(n), 0x1000_0000 + 0x111 * (n + 1))
cc1 = [a_rd(TIMER1 + t_cc(n)) for n in range(6)]
a_wr(TIMER3 + T_INTENSET, 0xFFFFFFFF)
inten3 = a_rd(TIMER3 + T_INTENSET)
a_wr(TIMER3 + T_INTENCLR, 0xFFFFFFFF)
a_wr(TIMER1 + T_INTENSET, 0xFFFFFFFF)
inten1 = a_rd(TIMER1 + T_INTENSET)
a_wr(TIMER1 + T_INTENCLR, 0xFFFFFFFF)
# One line: each compare in turn, enabled, raises line 26 and no other.
lines = []
a_wr(TIMER3 + T_START, 1)
for n in range(6):
    a_wr(TIMER3 + T_INTENCLR, 0xFFFFFFFF)
    a_wr(TIMER3 + t_event(n), 0)
    a_wr(NVIC_ICPR0, 0xFFFFFFFF)
    before = a_rd(NVIC_ISPR0)
    reader = 3 if n == 2 else 2
    a_wr(TIMER3 + T_INTENSET, 1 << (16 + n))
    target = (now3(reader) + 1_500_000) & 0xFFFFFFFF
    a_wr(TIMER3 + t_cc(n), target)
    early = a_rd(NVIC_ISPR0)
    while ((now3(reader) - target) & 0xFFFFFFFF) >= 0x8000_0000:
        time.sleep(0.2)
    lines.append({"compare": n, "pending_before": f"0x{before:08x}", "pending_while_armed": f"0x{early:08x}",
                  "pending_after": f"0x{a_rd(NVIC_ISPR0):08x}", "event": a_rd(TIMER3 + t_event(n))})
a_wr(TIMER3 + T_INTENCLR, 0xFFFFFFFF)
ok = (cc3 == [0x1000_0000 + 0x111 * (n + 1) for n in range(6)] and inten3 == 0x003F_0000
      and all(l["pending_before"] == "0x00000000" and l["pending_while_armed"] == "0x00000000"
              and l["pending_after"] == f"0x{1 << 26:08x}" and l["event"] == 1 for l in lines))
record("six_compares_one_line", "TIMER3 has six compare registers and one interrupt line",
       {"timer3_cc_read_back": [f"0x{v:08x}" for v in cc3], "timer3_intenset_read_back": f"0x{inten3:08x}",
        "timer1_cc_read_back_for_contrast": [f"0x{v:08x}" for v in cc1],
        "timer1_intenset_read_back_for_contrast": f"0x{inten1:08x}",
        "nvic_enabled_lines": f"0x{enabled:08x}", "per_compare": lines}, ok,
       "Each of the six compares, with its interrupt enabled, set the pending bit of line 26 (0x04000000) and no other.")

# P2: a compare written behind the counter does not fire before the wrap.
a_wr(NVIC_ICPR0, 0xFFFFFFFF)
a_wr(TIMER3 + t_event(0), 0)
c = now3()
behind = (c - 1_000) & 0xFFFFFFFF
a_wr(TIMER3 + t_cc(0), behind)
a_wr(TIMER3 + t_event(0), 0)
time.sleep(2.0)
later = now3()
event_behind = a_rd(TIMER3 + t_event(0))
ahead = (now3() + 1_500_000) & 0xFFFFFFFF
a_wr(TIMER3 + t_cc(0), ahead)
event_armed = a_rd(TIMER3 + t_event(0))
armed_at = now3()
while ((now3() - ahead) & 0xFFFFFFFF) >= 0x8000_0000:
    time.sleep(0.2)
event_ahead = a_rd(TIMER3 + t_event(0))
ok = event_behind == 0 and later - c > 1_900_000 and event_armed == 0 and armed_at < ahead and event_ahead == 1
record("compare_behind_the_counter", "A compare written behind the counter does not fire before the wrap",
       hexd({"counter_when_written": c, "compare_written": behind, "counter_at_the_check": later,
             "event_after_waiting": event_behind, "compare_then_written_ahead": ahead,
             "event_while_ahead": event_armed, "event_once_passed": event_ahead}), ok,
       "The counter ran on for over 1.9 s past the value with no event; the same register written ahead fired.")

# P3: an event flag is set when its interrupt is not enabled.
a_wr(TIMER3 + T_INTENCLR, 0xFFFFFFFF)
a_wr(TIMER3 + t_event(1), 0)
a_wr(NVIC_ICPR0, 0xFFFFFFFF)
inten = a_rd(TIMER3 + T_INTENSET)
target = (now3() + 1_500_000) & 0xFFFFFFFF
a_wr(TIMER3 + t_cc(1), target)
flag_before = a_rd(TIMER3 + t_event(1))
while ((now3() - target) & 0xFFFFFFFF) >= 0x8000_0000:
    time.sleep(0.2)
flag_after = a_rd(TIMER3 + t_event(1))
pending_disabled = a_rd(NVIC_ISPR0)
a_wr(TIMER3 + T_INTENSET, 1 << 17)
pending_enabled = a_rd(NVIC_ISPR0)
a_wr(TIMER3 + T_INTENCLR, 0xFFFFFFFF)
a_wr(TIMER3 + t_event(1), 0)
a_wr(NVIC_ICPR0, 0xFFFFFFFF)
ok = inten == 0 and flag_before == 0 and flag_after == 1 and pending_disabled == 0 and pending_enabled == 1 << 26
record("event_flag_without_interrupt", "An event flag is set when its interrupt is not enabled",
       hexd({"intenset": inten, "event_before": flag_before, "event_after": flag_after,
             "nvic_pending_with_interrupt_disabled": pending_disabled,
             "nvic_pending_once_enabled": pending_enabled}), ok,
       "With every TIMER3 interrupt disabled the compare set its event flag and left line 26 not pending; "
       "enabling the interrupt with the flag still set made the line pending.")

# P4: a GPIOTE task channel holds the pin whatever the port's direction
# register says. Pin P1.10 (atrial pace), read back through its input buffer.
PIN = 10
a_wr(pin_cnf(PIN), 0)  # direction input, input buffer connected, no pull
a_wr(P1 + P_OUTCLR, 1 << PIN)
a_wr(P1 + P_DIRCLR, 1 << PIN)
a_wr(g_config(0), 3 | (PIN << 8) | (1 << 13) | (3 << 16))  # task mode, port 1, initial low
def level(): return (a_rd(P1 + P_IN) >> PIN) & 1
steps = []
steps.append({"step": "channel configured, initial low; port: input", "dir_bit": (a_rd(P1 + P_DIR) >> PIN) & 1, "pin": level()})
a_wr(g_set(0), 1)
steps.append({"step": "SET task; port: input, OUT low", "dir_bit": (a_rd(P1 + P_DIR) >> PIN) & 1, "pin": level()})
a_wr(pin_cnf(PIN), 1 << 2)  # pull-down against the driven high
steps.append({"step": "pull-down enabled against the high", "dir_bit": (a_rd(P1 + P_DIR) >> PIN) & 1, "pin": level()})
a_wr(pin_cnf(PIN), 0)
a_wr(g_clr(0), 1)
steps.append({"step": "CLR task; port: input", "dir_bit": (a_rd(P1 + P_DIR) >> PIN) & 1, "pin": level()})
a_wr(P1 + P_OUTSET, 1 << PIN)
a_wr(P1 + P_DIRSET, 1 << PIN)
steps.append({"step": "port: output, OUT high, against the channel's low", "dir_bit": (a_rd(P1 + P_DIR) >> PIN) & 1, "pin": level()})
a_wr(pin_cnf(PIN), 3 << 2)  # pull-up against the driven low (direction bit of PIN_CNF: input)
steps.append({"step": "port: PIN_CNF input with pull-up, against the channel's low", "dir_bit": (a_rd(P1 + P_DIR) >> PIN) & 1, "pin": level()})
a_wr(g_set(0), 1)
a_wr(P1 + P_OUTCLR, 1 << PIN)
a_wr(P1 + P_DIRSET, 1 << PIN)
steps.append({"step": "SET task; port: output, OUT low", "dir_bit": (a_rd(P1 + P_DIR) >> PIN) & 1, "pin": level()})
a_wr(g_clr(0), 1)
a_wr(P1 + P_DIRCLR, 1 << PIN)
a_wr(pin_cnf(PIN), 0)
ok = [s["pin"] for s in steps] == [0, 1, 1, 0, 0, 0, 1]
record("gpiote_holds_the_pin", "A GPIOTE task channel holds the pin whatever the port's direction register says",
       {"pin": "P1.10", "steps": steps}, ok,
       "The pin followed the channel's SET and CLR tasks with the port set to input, and was not moved by the "
       "port's OUT and DIR written against it, nor by a pull against it.")

# P5: a PPI fork triggers a second task from one event. Compare 0 of TIMER3
# sets P1.10 (task) and P1.03 (fork); compare 4 clears both. P1.03 is a
# board-to-board line, so board B reads it as well.
MIRROR = 3
wr(B, pin_cnf(MIRROR), 1 << 2)  # board B: input, buffer connected, pull-down
a_wr(pin_cnf(MIRROR), 0)
a_wr(g_config(2), 3 | (MIRROR << 8) | (1 << 13) | (3 << 16))
a_wr(g_clr(0), 1)
a_wr(g_clr(2), 1)
a_wr(ppi_eep(0), TIMER3 + t_event(0)); a_wr(ppi_tep(0), g_set(0)); a_wr(ppi_fork(0), g_set(2))
a_wr(ppi_eep(1), TIMER3 + t_event(4)); a_wr(ppi_tep(1), g_clr(0)); a_wr(ppi_fork(1), g_clr(2))
a_wr(PPI_CHENSET, 0b11)
def pins():
    a = a_rd(P1 + P_IN)
    return {"a_p1_10": (a >> PIN) & 1, "a_p1_03": (a >> MIRROR) & 1, "b_p1_03": (rd(B, P1 + P_IN) >> MIRROR) & 1}
fork = [dict(step="both channels enabled, nothing fired", **pins())]
target = (now3() + 1_500_000) & 0xFFFFFFFF
a_wr(TIMER3 + t_cc(0), target)
while ((now3() - target) & 0xFFFFFFFF) >= 0x8000_0000:
    time.sleep(0.2)
fork.append(dict(step="compare 0 fired: SET by the task, SET by the fork", **pins()))
target = (now3() + 1_500_000) & 0xFFFFFFFF
a_wr(TIMER3 + t_cc(4), target)
while ((now3() - target) & 0xFFFFFFFF) >= 0x8000_0000:
    time.sleep(0.2)
fork.append(dict(step="compare 4 fired: CLR by the task, CLR by the fork", **pins()))
# The fork alone: the task end-point cleared, the fork kept.
a_wr(ppi_tep(0), 0)
target = (now3() + 1_500_000) & 0xFFFFFFFF
a_wr(TIMER3 + t_cc(0), target)
while ((now3() - target) & 0xFFFFFFFF) >= 0x8000_0000:
    time.sleep(0.2)
fork.append(dict(step="compare 0 fired with the task end-point cleared: the fork alone", **pins()))
a_wr(PPI_CHENCLR, 0b11)
a_wr(g_clr(0), 1); a_wr(g_clr(2), 1)
wr(B, pin_cnf(MIRROR), 2)  # board B's pin back to its reset configuration
ok = [(f["a_p1_10"], f["a_p1_03"], f["b_p1_03"]) for f in fork] == [(0, 0, 0), (1, 1, 1), (0, 0, 0), (0, 1, 1)]
record("ppi_fork", "A PPI fork triggers a second task from one event",
       {"channels": "CH0: TIMER3 COMPARE[0] -> GPIOTE SET[0] (P1.10), fork SET[2] (P1.03); "
                    "CH1: TIMER3 COMPARE[4] -> GPIOTE CLR[0], fork CLR[2]", "steps": fork}, ok,
       "One compare event moved both pins, and board B read the forked pin on the board-to-board line. "
       "Read through registers: that the two edges are simultaneous is not measured here.")

# P6: a timer in counter mode counts fork edges and raises its interrupt at
# a compare that is moved while it counts, the counter never cleared. Edges
# on P1.01 (the atrial sense line) are made by board B through its probe.
SENSE = 1
a_wr(pin_cnf(SENSE), 0)
wr(B, P1 + P_OUTCLR, 1 << SENSE)
wr(B, pin_cnf(SENSE), 1)  # board B: output
wr(B, P1 + P_DIRSET, 1 << SENSE)
a_wr(TIMER1 + T_STOP, 1); a_wr(TIMER1 + T_CLEAR, 1)
a_wr(TIMER1 + T_MODE, 1)  # counter
a_wr(TIMER1 + T_BITMODE, 3)
a_wr(TIMER1 + T_INTENCLR, 0xFFFFFFFF)
a_wr(g_config(3), 1 | (SENSE << 8) | (1 << 13) | (1 << 16))  # event mode, port 1, rising
a_wr(g_in(3), 0)
a_wr(ppi_eep(2), g_in(3)); a_wr(ppi_tep(2), TIMER3 + t_capture(3)); a_wr(ppi_fork(2), TIMER1 + T_COUNT)
a_wr(PPI_CHENSET, 1 << 2)
a_wr(TIMER1 + t_cc(0), 3)
a_wr(TIMER1 + t_event(0), 0)
a_wr(TIMER1 + T_INTENSET, 1 << 16)
a_wr(NVIC_ICPR0, 0xFFFFFFFF)
a_wr(TIMER1 + T_START, 1)
def count1():
    a_wr(TIMER1 + t_capture(1), 1)
    return a_rd(TIMER1 + t_cc(1))
def state(step):
    return {"step": step, "count": count1(), "compare": a_rd(TIMER1 + t_cc(0)), "event": a_rd(TIMER1 + t_event(0)),
            "line_9_pending": (a_rd(NVIC_ISPR0) >> 9) & 1, "timer3_capture": a_rd(TIMER3 + t_cc(3))}
def edge():
    wr(B, P1 + P_OUTSET, 1 << SENSE)
    wr(B, P1 + P_OUTCLR, 1 << SENSE)
counter = [state("compare at 3, no edge yet")]
for n in (1, 2, 3):
    edge()
    counter.append(state(f"edge {n}"))
# Moved while it counts: to count + 2, the event cleared, the counter not.
a_wr(TIMER1 + t_cc(0), 5)
a_wr(TIMER1 + t_event(0), 0)
a_wr(NVIC_ICPR0, 1 << 9)
counter.append(state("compare moved to 5, event cleared"))
for n in (4, 5, 6):
    edge()
    counter.append(state(f"edge {n}"))
a_wr(TIMER1 + T_INTENCLR, 0xFFFFFFFF)
a_wr(PPI_CHENCLR, 1 << 2)
wr(B, P1 + P_DIRCLR, 1 << SENSE)
wr(B, pin_cnf(SENSE), 2)
got = [(s["count"], s["event"], s["line_9_pending"]) for s in counter]
want = [(0, 0, 0), (1, 0, 0), (2, 0, 0), (3, 1, 1), (3, 0, 0), (4, 0, 0), (5, 1, 1), (6, 1, 1)]
captures = [s["timer3_capture"] for s in counter if s["step"].startswith("edge")]
ok = got == want and all(((b - a) & 0xFFFFFFFF) < 0x8000_0000 and b != a for a, b in zip(captures, captures[1:]))
record("counter_mode", "A timer in counter mode counts fork edges and raises its interrupt at a compare that is "
       "moved while it counts, with the counter never cleared",
       {"channel": "CH2: GPIOTE IN[3] (P1.01 rising) -> TIMER3 CAPTURE[3], fork TIMER1 COUNT", "steps": counter}, ok,
       "TIMER1 counted one per edge, raised line 9 at the edge that reached the compare, and after the compare "
       "was moved from 3 to 5 with the count left at 3, raised it again at the fifth edge and not the fourth. "
       "TIMER3's capture register took a new, later instant at every edge.")

reset_peripherals()
json.dump({"task": "1.2", "probes": results}, open(RECORD, "w"), indent=2)
print(f"{sum(r['verdict'] == 'confirmed' for r in results)} of {len(results)} confirmed; record: {RECORD}")
sys.exit(0 if all(r["verdict"] == "confirmed" for r in results) else 1)
