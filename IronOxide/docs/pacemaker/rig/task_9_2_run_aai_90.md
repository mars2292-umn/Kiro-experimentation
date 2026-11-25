# Task 9.2: measurement run, set aai-90

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,set-aai-90): sha256:6ac223f6c6f808b996230d9944ed71e563bfa97b3c2efd927d1597ea24d1d986, Generated_Config checksum 0x6db2ad66.
Scenario: nrf52840_measure, set aai-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 374 | 17713 / 276.8 | 450 | 0 | 0.351 | 2.000 | 374 | 2142 | 0 |
| `therapy.sense_v` | 375 | 15115 / 236.2 | 450 | 0 | 0.311 | 3.000 | 375 | 2164 | 0 |
| `therapy.sense_a` | 375 | 20779 / 324.7 | 450 | 0 | 0.400 | 4.000 | 375 | 2168 | 0 |
| `therapy.control` | 15000 | 30108 / 470.4 | 5000 | 0 | 1.038 | 20.000 | 0 | 0 | 5158 |
| `comms.link` | 6000 | 29442 / 460.0 | 2000 | 0 | 1.439 | 50.000 | 0 | 0 | 3386 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 22124 | 2854 / 44.6 |
| timer | 21522 | 3125 / 48.8 |
| intake | 1124 | 2311 / 36.1 |
| lock | 16124 | 456 / 7.1 |
| unlock | 16124 | 1821 / 28.5 |
| job_end | 22124 | 1978 / 30.9 |
| now | 31883 | 238 / 3.7 |
| dma | 83998 | 716 / 11.2 |
| epilogue | 22124 | 442 / 6.9 |
| view | 22124 | 403 / 6.3 |
| send | 5397 | 2643 / 41.3 |
| recv | 26397 | 1147 / 17.9 |

Hold (L_kernel): 4348 cycles (67.9 µs) over 246816 holds.

Events: 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

Verdict of the run: **pass** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).
