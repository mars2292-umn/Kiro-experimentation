# Task 9.2: measurement run, set doo-mit

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,set-env): sha256:45e8cc5059db08f6da274c6241dc7a5fc7ef3920942edf70528692496ab77176, Generated_Config checksum 0x6db2ad66.
Scenario: nrf52840_measure, set doo-mit, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 1749 | 15336 / 239.6 | 450 | 0 | 0.362 | 2.000 | 1749 | 2365 | 0 |
| `therapy.sense_v` | 375 | 17303 / 270.4 | 450 | 0 | 0.345 | 3.000 | 375 | 2164 | 0 |
| `therapy.sense_a` | 375 | 17427 / 272.3 | 450 | 0 | 0.347 | 4.000 | 375 | 2168 | 0 |
| `therapy.control` | 15000 | 30360 / 474.4 | 5000 | 0 | 1.042 | 20.000 | 0 | 0 | 5238 |
| `comms.link` | 6000 | 34657 / 541.5 | 2000 | 0 | 1.443 | 50.000 | 0 | 0 | 3386 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 23499 | 2914 / 45.5 |
| timer | 22895 | 3125 / 48.8 |
| intake | 2499 | 2458 / 38.4 |
| lock | 17499 | 456 / 7.1 |
| unlock | 17499 | 1822 / 28.5 |
| job_end | 23499 | 1978 / 30.9 |
| now | 22256 | 238 / 3.7 |
| dma | 83998 | 718 / 11.2 |
| epilogue | 23499 | 1218 / 19.0 |
| view | 23617 | 403 / 6.3 |
| send | 5398 | 2658 / 41.5 |
| recv | 26398 | 1161 / 18.1 |

Hold (L_kernel): 4348 cycles (67.9 µs) over 245439 holds.

Events: 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

Verdict of the run: **pass** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).
