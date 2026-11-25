# Task 9.2: measurement run, set vvi-90

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,set-vvi-90): sha256:4fcc624f6d20663a37f43225c35bf2dfd45892909a46e18f5880db8e69caf941, Generated_Config checksum 0x6db2ad66.
Scenario: nrf52840_measure, set vvi-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 375 | 17877 / 279.3 | 450 | 0 | 0.353 | 2.000 | 375 | 2142 | 0 |
| `therapy.sense_v` | 375 | 17415 / 272.1 | 450 | 0 | 0.365 | 3.000 | 375 | 2172 | 0 |
| `therapy.sense_a` | 374 | 18475 / 288.7 | 450 | 0 | 0.364 | 4.000 | 375 | 2172 | 0 |
| `therapy.control` | 15000 | 30116 / 470.6 | 5000 | 0 | 1.039 | 20.000 | 0 | 0 | 5370 |
| `comms.link` | 6000 | 29446 / 460.1 | 2000 | 0 | 1.439 | 50.000 | 0 | 0 | 3386 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 22124 | 2858 / 44.7 |
| timer | 21523 | 3125 / 48.8 |
| intake | 1125 | 2315 / 36.2 |
| lock | 16124 | 456 / 7.1 |
| unlock | 16124 | 1820 / 28.4 |
| job_end | 22124 | 1978 / 30.9 |
| now | 31758 | 238 / 3.7 |
| dma | 83998 | 716 / 11.2 |
| epilogue | 22124 | 442 / 6.9 |
| view | 22124 | 403 / 6.3 |
| send | 5397 | 2643 / 41.3 |
| recv | 26397 | 1147 / 17.9 |

Hold (L_kernel): 4348 cycles (67.9 µs) over 246693 holds.

Events: 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

Verdict of the run: **pass** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).
