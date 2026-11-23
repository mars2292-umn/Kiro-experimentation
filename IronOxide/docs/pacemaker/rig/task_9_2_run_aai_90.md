# Task 9.2: measurement run, set aai-90

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,set-aai-90): sha256:76f08043cd40f215cb9981138c8dceea572d8f2ac5cf6723c4ddd9f09957252d, Generated_Config checksum 0xec5fe19d.
Scenario: nrf52840_measure, set aai-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 374 | 14622 / 228.5 | 400 | 0 | 0.300 | 2.000 | 374 | 2056 | 0 |
| `therapy.sense_v` | 375 | 9687 / 151.4 | 400 | 0 | 0.223 | 3.000 | 375 | 2067 | 0 |
| `therapy.sense_a` | 375 | 14967 / 233.9 | 400 | 0 | 0.306 | 4.000 | 375 | 2066 | 0 |
| `therapy.control` | 15000 | 15310 / 239.2 | 5000 | 0 | 0.767 | 20.000 | 0 | 0 | 4476 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 16124 | 2748 / 42.9 |
| timer | 18522 | 2804 / 43.8 |
| intake | 1124 | 2208 / 34.5 |
| lock | 16124 | 438 / 6.8 |
| unlock | 16124 | 1691 / 26.4 |
| job_end | 16124 | 1756 / 27.4 |
| now | 31128 | 233 / 3.6 |
| epilogue | 16124 | 433 / 6.8 |
| view | 16124 | 403 / 6.3 |

Hold (L_kernel): 3560 cycles (55.6 µs) over 115269 holds.

Events: 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

Verdict of the run: **pass** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).
