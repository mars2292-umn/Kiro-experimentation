#!/usr/bin/env python3
"""Tasks 16.3 to 16.6 of docs/pacemaker/tasks.md: the rig runs of the
States of stage E3.

Usage: e3_states.py <record.json> [pace_now] [magnet] [battery] [reset]

Run from the workspace root with rsk's tools on PATH. Board A carries the
pulse generator (features `nrf52840,debug-port,quiet`: no set of its own,
so it starts in the Power-On Reset State), board B the Heart_Simulator in
its scenario `silent`, board C the bridge; the DCM_Stub (`dcm-stub do`)
gives the commands.

Every run captures both pace pins at 1 MHz. The capture is read from
sigrok-cli as it comes and each piece is stamped with the host's clock, so
that a sample has a host time: the latest it can have been taken. The
DCM_Stub stamps its commands with the same clock. A time from a command to
a pulse measured this way is the true time plus what the capture took to
reach the host, so it errs to the long side.

  pace_now  (16.3) Permanent VVI at 90 ppm; PACE_NOW; the time from the
            stub's send to the first pulse of 1.00 ms, against two cardiac
            cycles at the rate in force plus 500 ms (PM6.6); then every
            width against 1.00 ms +-0.02 ms and every interval against
            the Pace-Now LRL.
  magnet    (16.4) Permanent VVI at 60 ppm; the magnet pin raised through
            board B's probe; the battery level BOL, ERN, ERT by command;
            the pin lowered; the magnet response programmed Off and the
            pin raised again. The pace intervals of each phase.
  battery   (16.5) Permanent DDD at 60 ppm; the levels ERT, ERP, below the
            trip level and BOL again by command; temporary pacing asked
            for at ERT. The pulses of each Chamber in each phase.
  reset     (16.6) the pulse generator reset through its probe; the time
            from the reset to the first Pace, its width against 0.50 ms
            +-0.02 ms, and the intervals against the Power-On Reset LRL.

Exit 0 when every run chosen passed.
"""
import glob
import hashlib
import json
import os
import subprocess
import sys
import threading
import time

import numpy as np

CHIP = "nRF52840_xxAA"
BOARD_A = "1366:1060:001050217737"
BOARD_B = "1366:1061:001050279833"
BOARD_C = "1366:1061:001050211852"
TARGET = "thumbv7em-none-eabihf"
PG = f"target/pacemaker-link/{TARGET}/release/pacemaker"
HEART = f"target/heart-sim-silent/{TARGET}/release/heart-sim"
BRIDGE = f"tools/pm-bridge/target/{TARGET}/release/pm-bridge"
STUB = "target/debug/dcm-stub"
RATE = 1_000_000
ATRIAL, VENTRICULAR = 0, 1
P1 = 0x5000_0300
MAGNET_PIN = 7
STATES = {"por": 0, "permanent": 1, "temporary": 2, "pace_now": 3, "magnet": 4}
VVI, DDD = 8, 1
MODE_VVI, MODE_DDD = 42, 63
WIDTH_TOLERANCE_US = 20
INTERVAL_TOLERANCE_US = 8_000


def sh(cmd, cwd=None, check=True):
    out = subprocess.run(cmd, capture_output=True, text=True, cwd=cwd)
    if check and out.returncode != 0:
        sys.exit(f"{' '.join(cmd)}\n{out.stdout[-2000:]}{out.stderr[-2000:]}")
    return out


def sha(path):
    return "sha256:" + hashlib.sha256(open(path, "rb").read()).hexdigest()


def probe(sub, board, *rest):
    # A probe now and then fails to open with a USB transport error and
    # opens at the next try (seen 2026-11-24).
    for _ in range(3):
        out = sh(["probe-rs", sub, "--chip", CHIP, "--probe", board, *rest], check=False)
        if out.returncode == 0:
            return out
        time.sleep(1.0)
    return sh(["probe-rs", sub, "--chip", CHIP, "--probe", board, *rest])


def port():
    number = BOARD_C.split(":")[-1]
    ports = sorted(glob.glob(f"/dev/cu.usbmodem{number}*"))
    if not ports:
        sys.exit(f"no serial port of probe {number}")
    return ports[0]


def stub(*ops):
    """The DCM_Stub's operations; each one's record, with its host times."""
    out = sh([STUB, "do", port(), *ops], check=False)
    lines = [json.loads(l) for l in out.stdout.splitlines() if l.startswith("{")]
    for l in lines:
        print(f"    {'ok  ' if l['ok'] else 'FAIL'} {l['op']}: {json.dumps(l['detail'])[:200]}", flush=True)
    return lines


def magnet(on):
    """The magnet line (board B P1.07 to board A P1.07), driven through
    board B's probe; the host's clock around the write that moves it."""
    probe("write", BOARD_B, "b32", f"0x{P1 + 0x518:08x}", f"0x{1 << MAGNET_PIN:x}")
    before = time.time()
    probe("write", BOARD_B, "b32", f"0x{P1 + (0x508 if on else 0x50C):08x}", f"0x{1 << MAGNET_PIN:x}")
    return {"magnet": on, "before": before, "after": time.time()}


class Capture:
    """Both pace pins at 1 MHz for `seconds`, each piece stamped as it comes."""

    def __init__(self, seconds):
        self.proc = subprocess.Popen(["sigrok-cli", "-d", "fx2lafw", "--config", f"samplerate={RATE}", "--samples", str(RATE * seconds), "-O", "binary"],
                                     stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        self.pieces, self.stamps = [], []
        self.thread = threading.Thread(target=self._read)
        self.thread.start()

    def _read(self):
        total = 0
        while True:
            piece = self.proc.stdout.read1(1 << 16)
            if not piece:
                break
            total += len(piece)
            self.pieces.append(piece)
            self.stamps.append((total, time.time()))

    def started(self):
        """Blocks until the first samples have come."""
        while not self.stamps and self.proc.poll() is None:
            time.sleep(0.01)

    def finish(self):
        self.thread.join()
        self.proc.wait()
        self.data = np.frombuffer(b"".join(self.pieces), dtype=np.uint8)
        ends = np.array([s[0] for s in self.stamps], dtype=np.float64)
        arrived = np.array([s[1] for s in self.stamps], dtype=np.float64)
        # Sample i was taken no later than any later piece's arrival less
        # the samples between: the earliest such bound for sample 0.
        self.t0 = float(np.min(arrived - ends / RATE))
        return self

    def time_of(self, index):
        return self.t0 + index / RATE

    def pulses(self, bit):
        """(host time of the rise, sample of the rise, width in µs) of every
        whole pulse of the pin."""
        level = ((self.data >> bit) & 1).astype(np.int8)
        d = np.diff(level)
        rises, falls = np.flatnonzero(d == 1) + 1, np.flatnonzero(d == -1) + 1
        out = []
        for r in rises:
            k = np.searchsorted(falls, r)
            if k < len(falls):
                out.append((self.time_of(int(r)), int(r), int(falls[k] - r) * 1_000_000 // RATE))
        return out


def between(pulses, start, end):
    return [p for p in pulses if start <= p[0] < end]


def intervals(pulses):
    return [(b[1] - a[1]) * 1_000_000 // RATE for a, b in zip(pulses, pulses[1:])]


def spread(values):
    return {"count": len(values), "min": min(values) if values else None, "max": max(values) if values else None}


def all_near(values, want, tolerance):
    return bool(values) and all(abs(v - want) <= tolerance for v in values)


def start_pg():
    probe("reset", BOARD_C)
    probe("reset", BOARD_A)
    time.sleep(2.5)


def run_pace_now():
    start_pg()
    ops = stub("up", f"set:0={VVI},1=90", f"wait:state={STATES['permanent']}")
    ok = all(o["ok"] for o in ops)
    time.sleep(3)
    cap = Capture(24)
    cap.started()
    time.sleep(6)
    cmd = stub("pace-now", "status")
    time.sleep(1)
    cap.finish()
    sent = cmd[0]["sent"]
    v = cap.pulses(VENTRICULAR)
    before = between(v, 0, sent)
    after = between(v, sent, 1e12)
    first = next((p for p in after if abs(p[2] - 1000) <= WIDTH_TOLERANCE_US), None)
    # Two cardiac cycles at the rate in force when the command was sent, and 500 ms.
    cycle_us = intervals(before)[-1] if len(before) > 1 else None
    bound_ms = (2 * cycle_us / 1000 + 500) if cycle_us else None
    from_send_ms = (first[0] - sent) * 1000 if first else None
    pace_now = [p for p in after if first and p[1] >= first[1]]
    widths, gaps = [p[2] for p in pace_now], intervals(pace_now)
    stray = [p for p in after if first and p[1] < first[1]]
    good = (ok and cmd[0]["ok"] and cmd[0]["detail"]["accepted"] and cmd[1]["detail"]["state"] == STATES["pace_now"] and first is not None and bound_ms is not None
            and from_send_ms <= bound_ms and all_near(widths, 1000, WIDTH_TOLERANCE_US) and all_near(gaps, 923_076, INTERVAL_TOLERANCE_US)
            and all_near([p[2] for p in before], 400, WIDTH_TOLERANCE_US) and not cap.pulses(ATRIAL))
    out = {"ops": ops + cmd, "cycle_in_force_us": cycle_us, "bound_ms": bound_ms, "from_the_stubs_send_to_the_first_pace_now_pulse_ms": from_send_ms,
           "the_pulse_generators_share_ms": {"stated": "one link period to the transfer, one to link's Job, one control period, one Pace-Now LRL interval: 50 + 50 + 20 + 923"},
           "pulses_between_the_send_and_the_first_pace_now_pulse": [{"after_send_ms": (p[0] - sent) * 1000, "width_us": p[2]} for p in stray],
           "pace_now_widths_us": spread(widths), "pace_now_intervals_us": spread(gaps), "widths_before_us": spread([p[2] for p in before]),
           "state_after": cmd[1]["detail"], "verdict": "PASS" if good else "FAIL"}
    print(f"  pace_now: first Pace-Now pulse {from_send_ms and round(from_send_ms, 1)} ms after the send (bound {bound_ms}), widths {out['pace_now_widths_us']}, intervals {out['pace_now_intervals_us']}, stray {len(stray)}")
    return out


def phase_intervals(cap, pin, start, end, settle=2.6):
    """The pace intervals of a phase: of the pulses from `settle` seconds
    after its start (a change is in force at the next ventricular event,
    and the first interval after it is the new one) to its end."""
    return intervals(between(cap.pulses(pin), start + settle, end - 0.3))


def run_magnet():
    start_pg()
    magnet(False)
    ops = stub("up", "magnet-response:1", "battery:0", f"set:0={VVI},1=60", f"wait:state={STATES['permanent']}")
    time.sleep(2)
    cap = Capture(70)
    cap.started()
    marks = [("permanent VVI at 60 ppm", time.time(), 1_000_000)]
    time.sleep(6)
    m = magnet(True)
    marks.append(("magnet at BOL: 100 ppm", m["after"], 600_000))
    time.sleep(7)
    states = stub("status")
    ops += stub("battery:1")
    marks.append(("magnet at ERN: 90 ppm", time.time(), 666_666))
    time.sleep(8)
    ops += stub("battery:2")
    marks.append(("magnet at ERT: 85 ppm", time.time(), 705_882))
    time.sleep(8)
    m = magnet(False)
    marks.append(("magnet released: VVI at 60 ppm", m["after"], 1_000_000))
    time.sleep(8)
    states += stub("status")
    ops += stub("battery:0", "magnet-response:0")
    m = magnet(True)
    marks.append(("magnet response Off, magnet applied: VVI at 60 ppm", m["after"], 1_000_000))
    time.sleep(9)
    states += stub("status")
    m = magnet(False)
    end = time.time()
    cap.finish()
    ops += stub("magnet-response:1")
    phases, good = [], all(o["ok"] for o in ops)
    for (name, start, want), nxt in zip(marks, [x[1] for x in marks[1:]] + [end]):
        got = phase_intervals(cap, VENTRICULAR, start, nxt)
        ok = all_near(got, want, INTERVAL_TOLERANCE_US) and len(got) >= 3
        good = good and ok
        phases.append({"phase": name, "expected_us": want, "intervals_us": spread(got), "ok": ok})
        print(f"  magnet: {'ok  ' if ok else 'FAIL'} {name}: {spread(got)}")
    told = [s["detail"]["state"] for s in states]
    good = good and told == [STATES["magnet"], STATES["permanent"], STATES["permanent"]] and not cap.pulses(ATRIAL)
    widths = [p[2] for p in cap.pulses(VENTRICULAR)]
    return {"ops": ops, "phases": phases, "states_told_under_the_magnet_after_it_and_with_the_response_off": told, "widths_us": spread(widths),
            "verdict": "PASS" if good and all_near(widths, 400, WIDTH_TOLERANCE_US) else "FAIL"}


def run_battery():
    start_pg()
    ops = stub("up", "battery:0", f"set:0={DDD},1=60", f"wait:state={STATES['permanent']}")
    time.sleep(2)
    cap = Capture(64)
    cap.started()
    marks = [("BOL: DDD at 60 ppm", time.time())]
    time.sleep(7)
    ops += stub("battery:2")
    marks.append(("ERT: VVI", time.time()))
    time.sleep(4)
    at_ert = stub("status", f"temp:0={VVI},1=120", "status")
    time.sleep(5)
    ops += stub("battery:3")
    marks.append(("ERP: VVI", time.time()))
    time.sleep(8)
    at_erp = stub("status")
    ops += stub("battery:4")
    marks.append(("below the trip level: nothing", time.time()))
    time.sleep(8)
    tripped = stub("status", f"set:0={VVI},1=90")
    ops += stub("battery:0")
    marks.append(("BOL again: Power-On Reset, VVI at 65 ppm, 0.50 ms", time.time()))
    time.sleep(9)
    back = stub("status")
    end = time.time()
    cap.finish()
    a, v = cap.pulses(ATRIAL), cap.pulses(VENTRICULAR)
    bounds = [m[1] for m in marks] + [end]
    seg = lambda pulses, k, settle=2.6: between(pulses, bounds[k] + settle, bounds[k + 1] - 0.3)
    checks = []

    def check(name, ok, detail):
        checks.append({"check": name, "ok": bool(ok), "detail": detail})
        print(f"  battery: {'ok  ' if ok else 'FAIL'} {name}: {json.dumps(detail)[:220]}")

    check("BOL: both Chambers paced, the ventricle every 1000 ms, 150 ms after the atrium",
          all_near(intervals(seg(v, 0, 0.5)), 1_000_000, INTERVAL_TOLERANCE_US) and len(seg(a, 0, 0.5)) >= 4
          and all_near([(vp[1] - ap[1]) for ap, vp in zip(seg(a, 0, 0.5), [p for p in seg(v, 0, 0.5) if p[1] > seg(a, 0, 0.5)[0][1]])], 150_000, INTERVAL_TOLERANCE_US),
          {"ventricular_intervals_us": spread(intervals(seg(v, 0, 0.5))), "atrial_pulses": len(seg(a, 0, 0.5))})
    for k, level in ((1, "ERT"), (2, "ERP")):
        check(f"{level}: the ventricle alone, every 1000 ms", all_near(intervals(seg(v, k)), 1_000_000, INTERVAL_TOLERANCE_US) and not seg(a, k),
              {"ventricular_intervals_us": spread(intervals(seg(v, k))), "atrial_pulses": len(seg(a, k))})
    check("ERT: the Mode in force is VVI, the State Permanent", at_ert[0]["detail"]["mode"] == MODE_VVI and at_ert[0]["detail"]["state"] == STATES["permanent"], at_ert[0]["detail"])
    check("ERT: temporary pacing is declined and the State stays", at_ert[1]["detail"]["verdict"]["code"] == 8 and at_ert[2]["detail"]["state"] == STATES["permanent"],
          {"verdict": at_ert[1]["detail"]["verdict"], "after": at_ert[2]["detail"]})
    check("ERP: the Mode in force is VVI", at_erp[0]["detail"]["mode"] == MODE_VVI and at_erp[0]["detail"]["battery"] == 3, at_erp[0]["detail"])
    off_a, off_v = between(a, bounds[3] + 1.2, bounds[4] - 0.1), between(v, bounds[3] + 1.2, bounds[4] - 0.1)
    check("below the trip level: no pulse on either pin", not off_a and not off_v, {"atrial": len(off_a), "ventricular": len(off_v), "seconds": bounds[4] - bounds[3] - 1.3})
    check("below the trip level: the State is Power-On Reset and a command is declined",
          tripped[0]["detail"]["state"] == STATES["por"] and tripped[0]["detail"]["battery"] == 4 and tripped[1]["detail"]["verdict"]["code"] == 8,
          {"status": tripped[0]["detail"], "verdict_on_a_set": tripped[1]["detail"]["verdict"]})
    por = seg(v, 4, 0.0)
    first_after_ms = (por[0][0] - bounds[4]) * 1000 if por else None
    check("BOL again: Power-On Reset pacing, VVI at 65 ppm, 0.50 ms wide, the atrium not paced",
          all_near(intervals(por), 923_076, INTERVAL_TOLERANCE_US) and all_near([p[2] for p in por], 500, WIDTH_TOLERANCE_US) and not seg(a, 4, 0.0)
          and back[0]["detail"]["state"] == STATES["por"] and back[0]["detail"]["mode"] == MODE_VVI,
          {"intervals_us": spread(intervals(por)), "widths_us": spread([p[2] for p in por]), "first_pulse_ms_after_the_level_was_sent": first_after_ms, "status": back[0]["detail"]})
    good = all(o["ok"] for o in ops) and all(c["ok"] for c in checks)
    return {"ops": ops, "phases": [m[0] for m in marks], "checks": checks, "verdict": "PASS" if good else "FAIL"}


def reset_marks(cap, since):
    """The ventricular pin after a reset: the stretches in which it was not
    driven (the analyzer reads an undriven pin high; longer than any Pace
    of this run, 0.5 ms), and the Paces after the last of them."""
    v = between(cap.pulses(VENTRICULAR), since, 1e12)
    floats = [p for p in v if p[2] > 600]
    paces = [p for p in v if p[2] <= 600 and (not floats or p[1] > floats[-1][1])]
    return floats, paces


def run_reset():
    """From a reset until `control`'s first Job configures them the pace
    pins are not driven, and the analyzer reads them high: the capture
    shows the reset itself. The time from a reset to the first Pace is
    measured in two parts, because no reset through the probe leaves the
    whole second undisturbed: a probe that detaches holds the core some
    57 ms into the run, `control` misses a deadline and the Partition
    starts a second time (finding B10), and a probe that stays attached
    holds the core in reset for a time of its own before the run.

    1. The core's own system reset, asked for by one write to its reset
       register: from the reset to the pins driven low (the Kernel's boot
       and `control`'s first Job, which is over before the probe detaches).
    2. A start under an attached `probe-rs run`, which nothing disturbs:
       from the pins driven low to the first Pace.

    The time from a reset to the first Pace is the sum."""
    start_pg()
    cap = Capture(8)
    cap.started()
    time.sleep(3)
    before = time.time()
    # AIRCR: VECTKEY and SYSRESETREQ.
    subprocess.run(["probe-rs", "write", "--chip", CHIP, "--probe", BOARD_A, "b32", "0xE000ED0C", "0x05FA0004"], capture_output=True, text=True)
    cap.finish()
    floats1, paces1 = reset_marks(cap, before - 0.5)
    undriven_us = floats1[-1][2] if len(floats1) == 1 else None
    disturbed_ms = (paces1[0][1] - floats1[-1][1]) * 1000 / RATE if paces1 and floats1 else None

    # Long enough for the flashing, which takes up to 12 s, and six Paces.
    cap = Capture(22)
    cap.started()
    time.sleep(1)
    before = time.time()
    run = subprocess.Popen(["probe-rs", "run", "--chip", CHIP, "--probe", BOARD_A, "--disable-progressbars", "--catch-hardfault", PG],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    cap.finish()
    run.kill()
    run.wait()
    floats2, paces2 = reset_marks(cap, before)
    driven = floats2[-1][1] + floats2[-1][2] * RATE // 1_000_000 if floats2 else None
    driven_to_first_ms = (paces2[0][1] - driven) * 1000 / RATE if paces2 and driven is not None else None
    widths, gaps = [p[2] for p in paces2], intervals(paces2)
    total_ms = undriven_us / 1000 + driven_to_first_ms if undriven_us is not None and driven_to_first_ms is not None else None
    # The stated bound: the pins driven within 20 ms of the reset (the
    # Kernel's boot and `control`'s first Job), and the first Pace one
    # Power-On Reset LRL interval after that Job, with the tolerance of an
    # interval.
    bound_ms = 20 + 923.077 + INTERVAL_TOLERANCE_US / 1000
    good = (total_ms is not None and total_ms <= bound_ms and undriven_us <= 20_000 and all_near(widths, 500, WIDTH_TOLERANCE_US)
            and all_near(gaps, 923_076, INTERVAL_TOLERANCE_US) and len(paces2) >= 6)
    print(f"  reset: pins undriven {undriven_us} us after the reset; first Pace {driven_to_first_ms} ms after the pins were driven; sum {total_ms} ms (bound {bound_ms}); "
          f"widths {spread(widths)}, intervals {spread(gaps)}; with the detaching probe {disturbed_ms} ms")
    return {"part_1": {"reset": "the core's system reset (one write to its reset register through board A's probe)", "resets_seen_on_the_pin": len(floats1),
                       "pins_undriven_after_the_reset_us": undriven_us,
                       "first_pace_ms_after_the_reset_with_the_probe_detaching": disturbed_ms},
            "part_2": {"start": "under an attached probe-rs run", "undriven_stretches_seen": len(floats2), "first_pace_ms_after_the_pins_were_driven": driven_to_first_ms,
                       "widths_us": spread(widths), "intervals_us": spread(gaps), "paces": len(paces2)},
            "first_pace_ms_after_a_reset": total_ms, "bound_ms": bound_ms, "verdict": "PASS" if good else "FAIL"}


RUNS = {"pace_now": ("16.3", run_pace_now), "magnet": ("16.4", run_magnet), "battery": ("16.5", run_battery), "reset": ("16.6", run_reset)}


def main():
    record = sys.argv[1]
    chosen = [a for a in sys.argv[2:] if a in RUNS] or list(RUNS)
    sh(["cargo", "build", "-q", "-p", "dcm-stub"])
    sh(["cargo", "build", "-q", "--release"], cwd="tools/pm-bridge")
    sh(["cargo", "build", "-q", "-p", "pacemaker", "--release", "--target", TARGET, "--no-default-features", "--features",
        "nrf52840,debug-port,quiet", "--target-dir", "target/pacemaker-link"])
    sh(["cargo", "build", "-q", "-p", "heart-sim", "--release", "--target", TARGET, "--no-default-features", "--features",
        "nrf52840,debug-port,quiet,scenario-silent", "--target-dir", "target/heart-sim-silent"])
    kept = json.load(open(record)) if os.path.exists(record) else {"runs": {}}
    kept["boards"] = {"A": BOARD_A, "B": BOARD_B, "C": BOARD_C}
    kept["binaries"] = {"pacemaker": sha(PG), "heart-sim": sha(HEART), "pm-bridge": sha(BRIDGE)}
    kept["capture"] = {"rate_hz": RATE, "pins": "D0 the atrial pace pin, D1 the ventricular", "width_tolerance_us": WIDTH_TOLERANCE_US, "interval_tolerance_us": INTERVAL_TOLERANCE_US}
    probe("download", BOARD_C, "--disable-progressbars", BRIDGE)
    probe("download", BOARD_B, "--disable-progressbars", HEART)
    probe("reset", BOARD_B)
    probe("download", BOARD_A, "--disable-progressbars", PG)
    ok = True
    for name in chosen:
        task, run = RUNS[name]
        print(f"{name} (task {task})", flush=True)
        out = run()
        out["task"] = task
        kept["runs"][name] = out
        json.dump(kept, open(record, "w"), indent=1)
        print(f"{name}: {out['verdict']}", flush=True)
        ok = ok and out["verdict"] == "PASS"
    magnet(False)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
