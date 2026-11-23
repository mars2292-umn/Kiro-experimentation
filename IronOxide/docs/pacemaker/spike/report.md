# Hardware spike: report

**PACEMAKER on rsk, task 1 (design open point PD-O1)**

The spike is `spikes/pm-timer`: a privileged bare-metal program for board A in its own Cargo workspace, outside rsk's. This report grows with tasks 1.2 to 1.4. Every number here is from a run named beside it.

## Task 1.1: pulse widths made by hardware (2026-11-22)

**What ran.** TIMER3 at 1 MHz. Compares 0 and 1 set the atrial and the ventricular pace pin, compares 4 and 5 clear them, through four PPI channels to GPIOTE set and clear tasks. The main loop writes the four compare registers 1 ms ahead and waits; it makes no edge itself. Groups of 200 pulses per pin, one group per width.

**Bench.** Board A (J-Link 001050217737). Atrial pace pin P1.10 on analyzer D0, ventricular P1.11 on D1. Analyzer fx2lafw through sigrok-cli 0.7.2 at 4 MHz (a sample is 0.25 µs), one capture of 15 s. Binary SHA-256 `1e30d5086533886b322c31d1e30dadd2f67ee0dd7054513d00fbdb7cba183169` (rustc 1.95.0, release), flashed with probe-rs 0.32.0. Measured by `spikes/pm-timer/measure.py`; the record is `task_1_1_widths.json`.

**Result: pass.** 5,527 complete pulses, every one within one analyzer sample of its programmed width.

| Programmed | Tolerance (PM-PAR-05) | Atrial: pulses, min to max | Ventricular: pulses, min to max | Worst error |
|---|---|---|---|---|
| 0.05 ms | 0.2 ms | 578, 50.00 to 50.25 µs | 579, 50.00 to 50.25 µs | 0.25 µs |
| 0.4 ms | 0.2 ms | 600, 400.00 to 400.25 µs | 600, 400.00 to 400.25 µs | 0.25 µs |
| 0.5 ms (Power-On Reset) | ±0.02 ms | 600, 500.00 to 500.25 µs | 600, 499.75 to 500.00 µs | 0.25 µs |
| 1.00 ms (Pace-Now) | ±0.02 ms | 585, 1000.00 to 1000.25 µs | 585, 999.75 to 1000.00 µs | 0.25 µs |
| 1.9 ms | 0.2 ms | 400, 1899.75 to 1900.00 µs | 400, 1899.75 to 1900.00 µs | 0.25 µs |

No pulse fell outside the five widths. The spread is the analyzer's sample period, so the measurement shows no error of the pulse itself.

**What this confirms of the design.** A compare of TIMER3 acting on a pin through PPI and a GPIOTE task gives a pulse width that does not depend on software (PD-01, PM28.2). Compare registers 4 and 5 of TIMER3 work as compares 0 and 1 do. While the spike ran, the port's direction register read zero for the pace pins (`P1.DIR = 0x00000000`, read through the debug probe): the GPIOTE task channel drives the pin without the port being configured as an output.

**What it does not show.** The interval between pulses and the arming sequence of the design under interference (task 1.3). Forks, counter mode, the event flag without its interrupt, and a compare written behind the counter are task 1.2, below. The analyzer's own clock was not calibrated; an error of its rate would scale every width alike.

## Task 1.2: the register behaviours the design relies on (2026-11-22)

**What ran.** The spike built with its feature `idle` (binary SHA-256 `1baed16641e2d883b547d650364d48944f015845aff568996baaffca4f1abe66`): it reopens the debug port, starts the crystal oscillator and spins. The host script `spikes/pm-timer/probe_1_2.py` then wrote and read every register through the debug probes with probe-rs 0.32.0; the peripherals run without the core. Board B was used to drive the atrial sense line (P1.01) and to read P1.03. No interrupt was enabled in the core (`NVIC ISER0 = 0`), so a line's pending bit shows what a peripheral raised. The record is `task_1_2_probes.json`, with the reads behind each row.

**Result: six of six confirmed.**

| Behaviour | Verdict | What was read |
|---|---|---|
| TIMER3 has six compare registers and one interrupt line | Confirmed | Six distinct values written to `CC[0]` to `CC[5]` read back unchanged; `INTENSET` read back `0x003f0000`. Each of the six compares in turn, enabled and armed 1.5 s ahead, set its event flag and the pending bit of line 26 (`ISPR0 = 0x04000000`) and of no other line. For contrast, TIMER1 kept four values, read `CC[4]` and `CC[5]` as zero and `INTENSET` as `0x000f0000` |
| A compare written behind the counter does not fire before the wrap | Confirmed | `CC[0]` written 1,000 counts behind the counter: no event after the counter had run a further 2.7 million counts (2.7 s). The same register written 1.5 s ahead: no event while ahead, the event once passed |
| An event flag is set when its interrupt is not enabled | Confirmed | With `INTENSET = 0`, compare 1 set `EVENTS_COMPARE[1]` and line 26 stayed not pending; enabling the interrupt with the flag still set made the line pending |
| A GPIOTE task channel holds the pin whatever the port's direction register says | Confirmed | P1.10 followed the channel's SET and CLR tasks with the port's direction bit 0. It was not moved by the port's `OUT` and `DIR` written against it (output high against the channel's low, output low against its high) nor by a pull-down against its high or a pull-up against its low |
| A PPI fork triggers a second task from one event | Confirmed | Compare 0 through one channel set P1.10 (task) and P1.03 (fork); compare 4 through another cleared both; with the task end-point cleared, the fork alone set P1.03. Board B read P1.03 high and low on the board-to-board line each time |
| A timer in counter mode counts fork edges and raises its interrupt at a compare that is moved while it counts, the counter never cleared | Confirmed | A rising edge on P1.01 captured TIMER3 (task) and counted TIMER1 (fork). TIMER1 read 1, 2, 3 after three edges and raised line 9 at the third (compare 3) and not before. With the compare moved to 5 and the count left at 3, the fourth edge raised nothing and the fifth raised line 9; the sixth read 6. TIMER3's capture register held a new, later instant after every edge |

**What this settles in the design.** The hardware signal flow of `design.md` stands as drawn: six compares on one interrupt line for TIMER3, sense edges counted by TIMER1 and TIMER2 through forks with a compare that is moved and never cleared, `pace` finding which Chambers paced by event flags whose interrupts are off, and GPIOTE task pins that the port's registers cannot move. The second hazard of "Arming and the lead" is real: a compare written behind the counter waits for the wrap.

It also settles how the pace pulses reach the Heart_Simulator (the open point of task 4.1): a fork of each pace channel drives a copy of the pace pin on a board-to-board line, P1.03 for the atrium and P1.04 for the ventricle. The design is changed accordingly (Draft 0.3).

**What it does not show.** That the task and the fork act at the same instant: both pins were read through registers, hundreds of milliseconds after the event, and the analyzer does not see P1.03. The write order of "Arming and the lead" under interference and the value of `LEAD` (task 1.3). The first run of the script reported compare 2 as refuted; the cause was the script, which read the counter through capture register 2 and so overwrote the compare under test. It was corrected to read through another register and the whole set was run again; the record is of the second run.

## A bench finding: the debug port

The first build of the spike locked board A's debug port. This silicon protects the port at every reset, and a program has to reopen it by writing APPROTECT.DISABLE first thing, as rsk's HIL builds do (Verification_Plan, "Access port protection"). The spike did not, so the next probe command after its reset found the core locked.

Recovery followed the bench procedure of the Verification_Plan, on board A only: an unlock by full erase (`probe-rs download --allow-erase-all`), then UICR.APPROTECT restored (0x5A at 0x10001208). The spike now writes APPROTECT.DISABLE at its start, and the port was confirmed open after two further resets. The widths above are from the corrected binary. Board B was not flashed or erased.

Any bare-metal program put on a rig board must make that write.

## Task 4.1: the lines the Application needs (2026-11-22)

Measured without rewiring; recorded in `hil-rig.toml` under `[wiring]` with the prefix `pacemaker_`.

| Signal | Line | How it was measured |
|---|---|---|
| Atrial pace to the analyzer | A P1.10 to D0 | The spike's pulses, above |
| Ventricular pace to the analyzer | A P1.11 to D1 | The spike's pulses, above |
| Atrial sense, B to A | P1.01 | Driven from B through its debug probe, read on A; and from A, read on B |
| Ventricular sense, B to A | P1.02 | The same |
| Spare board-to-board lines | P1.03, P1.04 | The same |

Each of the four board-to-board lines reached its own pin on the other board and no other, in both directions, and all read low when none was driven. Board B was reset afterwards so that its application restarts.

**Since then (2026-11-22).** The pace pins the analyzer sees do not reach board B, and the analyzer cannot see the board-to-board wires, so the Heart_Simulator receives the pace pulses on P1.03 and P1.04, driven by a fork of each pace channel; task 1.2's fork probe confirmed it. The magnet wire was added (B P1.07 to A P1.07) and measured in both directions. The bridge on SPIM1 is not wired; it is needed from stage E3.
