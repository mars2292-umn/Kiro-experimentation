# Task 9.2: measurement run, set aai-90

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,set-aai-90): sha256:1bec2176fb04a6c6d824d21a34f9edc57640f67f04a1022f0b05288a6041877b, Generated_Config checksum 0x6db2ad66.
Scenario: nrf52840_measure, set aai-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 375 | 16445 / 257.0 | 450 | 0 | 0.331 | 2.000 | 375 | 2146 | 0 |
| `therapy.sense_v` | 375 | 17359 / 271.2 | 450 | 0 | 0.346 | 3.000 | 375 | 2164 | 0 |
| `therapy.sense_a` | 375 | 19823 / 309.7 | 450 | 0 | 0.385 | 4.000 | 375 | 2168 | 0 |
| `therapy.control` | 15000 | 29196 / 456.2 | 5000 | 0 | 1.024 | 20.000 | 0 | 0 | 4610 |
| `comms.link` | 6000 | 29506 / 461.0 | 2000 | 0 | 1.425 | 50.000 | 0 | 0 | 3386 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 22125 | 2858 / 44.7 |
| timer | 21521 | 3125 / 48.8 |
| intake | 1125 | 2311 / 36.1 |
| lock | 16125 | 456 / 7.1 |
| unlock | 16125 | 1822 / 28.5 |
| job_end | 22125 | 1981 / 31.0 |
| now | 31883 | 238 / 3.7 |
| dma | 83998 | 718 / 11.2 |
| epilogue | 22125 | 442 / 6.9 |
| view | 22125 | 403 / 6.3 |
| send | 5397 | 2658 / 41.5 |
| recv | 26397 | 1162 / 18.2 |

Hold (L_kernel): 4348 cycles (67.9 µs) over 246820 holds.

Events: 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

Verdict of the run: **pass** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).
