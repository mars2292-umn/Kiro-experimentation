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

**What it does not show.** The interval between pulses, the arming sequence of the design under interference (task 1.3), forks, counter mode, the event flag without its interrupt, and a compare written behind the counter (task 1.2). The analyzer's own clock was not calibrated; an error of its rate would scale every width alike.

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

**Open.** The pace pins the analyzer sees do not reach board B, and the analyzer cannot see the board-to-board wires. Proposal: the Heart_Simulator receives the pace pulses on P1.03 and P1.04, driven by a second GPIOTE channel per Chamber from the same compare events (a fork of each pace channel). This needs no wire; it adds four forks and two GPIOTE channels to the design, and task 1.2's fork probe decides whether it stands. The magnet pin and the bridge on SPIM1 are not wired; they are needed from stage E3.
