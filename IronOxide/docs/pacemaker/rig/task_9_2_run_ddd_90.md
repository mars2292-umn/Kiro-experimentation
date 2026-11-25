# Task 9.2: measurement run, set ddd-90

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,set-env): sha256:49fd89a6d9e7de2bebda032cbf43d3a63f8d78b9bc106d8684e00771f8a7de5b, Generated_Config checksum 0x6db2ad66.
Scenario: nrf52840_measure, set ddd-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 751 | 18437 / 288.1 | 450 | 0 | 0.360 | 2.000 | 751 | 2150 | 0 |
| `therapy.sense_v` | 375 | 18527 / 289.5 | 450 | 0 | 0.364 | 3.000 | 375 | 2168 | 0 |
| `therapy.sense_a` | 375 | 17631 / 275.5 | 450 | 0 | 0.351 | 4.000 | 375 | 2168 | 0 |
| `therapy.control` | 15000 | 30356 / 474.3 | 5000 | 0 | 1.042 | 20.000 | 0 | 0 | 5242 |
| `comms.link` | 6000 | 29506 / 461.0 | 2000 | 0 | 1.443 | 50.000 | 0 | 0 | 4566 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 22501 | 2858 / 44.7 |
| timer | 21898 | 3125 / 48.8 |
| intake | 1501 | 2311 / 36.1 |
| lock | 16501 | 456 / 7.1 |
| unlock | 16501 | 1822 / 28.5 |
| job_end | 22501 | 1981 / 31.0 |
| now | 20645 | 238 / 3.7 |
| dma | 83998 | 718 / 11.2 |
| epilogue | 22501 | 443 / 6.9 |
| view | 22501 | 403 / 6.3 |
| send | 5398 | 2658 / 41.5 |
| recv | 26398 | 1161 / 18.1 |

Hold (L_kernel): 4348 cycles (67.9 µs) over 237841 holds.

Events: 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

Verdict of the run: **pass** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).
