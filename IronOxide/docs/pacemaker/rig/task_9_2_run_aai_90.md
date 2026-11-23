# Task 9.2: measurement run, set aai-90

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,set-aai-90): sha256:a508f969a7239f3338e45a63209351554049bafd81399446dbd394d94e0d4614, Generated_Config checksum 0x2d3eb664.
Scenario: nrf52840_measure, set aai-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 374 | 11446 / 178.8 | 350 | 0 | 0.300 | 2.000 | 374 | 2052 | 0 |
| `therapy.sense_v` | 375 | 9687 / 151.4 | 350 | 0 | 0.274 | 3.000 | 375 | 2067 | 0 |
| `therapy.sense_a` | 375 | 14975 / 234.0 | 350 | 0 | 0.306 | 4.000 | 375 | 2066 | 0 |
| `therapy.control` | 15000 | 17550 / 274.2 | 5000 | 0 | 0.767 | 20.000 | 0 | 0 | 5408 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 16124 | 2756 / 43.1 |
| timer | 18500 | 2844 / 44.4 |
| intake | 1124 | 2208 / 34.5 |
| lock | 16124 | 438 / 6.8 |
| unlock | 16124 | 1691 / 26.4 |
| job_end | 16124 | 1756 / 27.4 |
| now | 31128 | 233 / 3.6 |
| epilogue | 16124 | 434 / 6.8 |
| view | 16124 | 403 / 6.3 |

Hold (L_kernel): 3669 cycles (57.3 µs) over 115247 holds.

Events: 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

Verdict of the run: **pass** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).
