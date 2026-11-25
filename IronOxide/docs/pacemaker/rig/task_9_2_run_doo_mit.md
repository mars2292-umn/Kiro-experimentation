# Task 9.2: measurement run, set doo-mit

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,set-env): sha256:fda44dc976c216a54f74ae94a72e04136f8e943f754f4b3ae4107f190eec400c, Generated_Config checksum 0x6db2ad66.
Scenario: nrf52840_measure, set doo-mit, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 1749 | 16584 / 259.1 | 450 | 0 | 0.338 | 2.000 | 1749 | 2282 | 0 |
| `therapy.sense_v` | 375 | 18575 / 290.2 | 450 | 0 | 0.365 | 3.000 | 375 | 2164 | 0 |
| `therapy.sense_a` | 375 | 15111 / 236.1 | 450 | 0 | 0.363 | 4.000 | 375 | 2168 | 0 |
| `therapy.control` | 15000 | 31232 / 488.0 | 5000 | 0 | 1.056 | 20.000 | 0 | 0 | 5086 |
| `comms.link` | 6000 | 34505 / 539.1 | 2000 | 0 | 1.457 | 50.000 | 0 | 0 | 3386 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 23499 | 2930 / 45.8 |
| timer | 22894 | 3125 / 48.8 |
| intake | 2499 | 2374 / 37.1 |
| lock | 17499 | 456 / 7.1 |
| unlock | 17499 | 1822 / 28.5 |
| job_end | 23499 | 1979 / 30.9 |
| now | 22256 | 238 / 3.7 |
| dma | 83998 | 718 / 11.2 |
| epilogue | 23499 | 1218 / 19.0 |
| view | 23616 | 403 / 6.3 |
| send | 5397 | 2643 / 41.3 |
| recv | 26397 | 1149 / 18.0 |

Hold (L_kernel): 4348 cycles (67.9 µs) over 245436 holds.

Events: 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

Verdict of the run: **pass** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).
