# Task 9.2: measurement run, set ddd-90

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,set-env): sha256:e04053954067eccf2b219dfaac6f4c63f8fbede33a9dce0be88ec367882f139a, Generated_Config checksum 0x6db2ad66.
Scenario: nrf52840_measure, set ddd-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 750 | 19849 / 310.1 | 450 | 0 | 0.384 | 2.000 | 750 | 2150 | 0 |
| `therapy.sense_v` | 375 | 19443 / 303.8 | 450 | 0 | 0.379 | 3.000 | 375 | 2164 | 0 |
| `therapy.sense_a` | 375 | 18655 / 291.5 | 450 | 0 | 0.367 | 4.000 | 375 | 2168 | 0 |
| `therapy.control` | 15000 | 31208 / 487.6 | 5000 | 0 | 1.056 | 20.000 | 0 | 0 | 4410 |
| `comms.link` | 6000 | 29446 / 460.1 | 2000 | 0 | 1.456 | 50.000 | 0 | 0 | 3386 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 22500 | 2858 / 44.7 |
| timer | 21895 | 3125 / 48.8 |
| intake | 1500 | 2311 / 36.1 |
| lock | 16500 | 456 / 7.1 |
| unlock | 16500 | 1822 / 28.5 |
| job_end | 22500 | 1978 / 30.9 |
| now | 20636 | 238 / 3.7 |
| dma | 83998 | 716 / 11.2 |
| epilogue | 22500 | 442 / 6.9 |
| view | 22500 | 403 / 6.3 |
| send | 5398 | 2643 / 41.3 |
| recv | 26398 | 1147 / 17.9 |

Hold (L_kernel): 4348 cycles (67.9 µs) over 237824 holds.

Events: 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

Verdict of the run: **pass** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).
