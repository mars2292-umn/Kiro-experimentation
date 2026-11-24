# Task 9.2: measurement run, set aai-90

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,set-aai-90): sha256:e578c9f4ae2442e77adf57ad079a1c0a52df23cfddffdaee6716d2f1a8400972, Generated_Config checksum 0x2d3eb664.
Scenario: nrf52840_measure, set aai-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 374 | 14162 / 221.3 | 350 | 0 | 0.292 | 2.000 | 374 | 2052 | 0 |
| `therapy.sense_v` | 375 | 12855 / 200.9 | 350 | 0 | 0.275 | 3.000 | 375 | 2067 | 0 |
| `therapy.sense_a` | 375 | 14923 / 233.2 | 350 | 0 | 0.358 | 4.000 | 375 | 2074 | 0 |
| `therapy.control` | 15000 | 15094 / 235.8 | 5000 | 0 | 0.764 | 20.000 | 0 | 0 | 6389 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 16124 | 2784 / 43.5 |
| timer | 18522 | 2804 / 43.8 |
| intake | 1124 | 2216 / 34.6 |
| lock | 16124 | 439 / 6.9 |
| unlock | 16124 | 1691 / 26.4 |
| job_end | 16124 | 1862 / 29.1 |
| now | 31128 | 233 / 3.6 |
| epilogue | 16124 | 434 / 6.8 |
| view | 16124 | 403 / 6.3 |

Hold (L_kernel): 3560 cycles (55.6 µs) over 115269 holds.

Events: 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

Verdict of the run: **pass** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).
