# Task 9.2: measurement run, set vvi-90

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,set-vvi-90): sha256:c2a3cc1ff0eb5b90d471af3d047a28853bc20d6a1a6c0dcd6cf048597ae84370, Generated_Config checksum 0x2d3eb664.
Scenario: nrf52840_measure, set vvi-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 375 | 14470 / 226.1 | 350 | 0 | 0.295 | 2.000 | 375 | 2052 | 0 |
| `therapy.sense_v` | 375 | 14939 / 233.4 | 350 | 0 | 0.305 | 3.000 | 375 | 2067 | 0 |
| `therapy.sense_a` | 375 | 13007 / 203.2 | 350 | 0 | 0.275 | 4.000 | 375 | 2066 | 0 |
| `therapy.control` | 15000 | 15298 / 239.0 | 5000 | 0 | 0.767 | 20.000 | 0 | 0 | 5448 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 16125 | 2748 / 42.9 |
| timer | 18522 | 2804 / 43.8 |
| intake | 1125 | 2208 / 34.5 |
| lock | 16125 | 438 / 6.8 |
| unlock | 16125 | 1690 / 26.4 |
| job_end | 16125 | 1756 / 27.4 |
| now | 31004 | 233 / 3.6 |
| epilogue | 16125 | 434 / 6.8 |
| view | 16125 | 403 / 6.3 |

Hold (L_kernel): 3560 cycles (55.6 µs) over 115150 holds.

Events: 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

Verdict of the run: **pass** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).
