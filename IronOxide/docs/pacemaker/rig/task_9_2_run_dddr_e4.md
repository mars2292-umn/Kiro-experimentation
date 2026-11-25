# Task 9.2: measurement run, set dddr-e4

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,set-env): sha256:60ead83ddf86dae0e1b7e9bb8e410c57411cbe03895812a163f36ebf4e599174, Generated_Config checksum 0x6db2ad66.
Scenario: nrf52840_measure, set dddr-e4, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 804 | 19253 / 300.8 | 450 | 0 | 0.572 | 2.000 | 804 | 2361 | 0 |
| `therapy.sense_v` | 375 | 19215 / 300.2 | 450 | 0 | 0.375 | 3.000 | 375 | 2164 | 0 |
| `therapy.sense_a` | 375 | 17719 / 276.9 | 450 | 0 | 0.352 | 4.000 | 375 | 2168 | 0 |
| `therapy.control` | 15000 | 35207 / 550.1 | 5000 | 0 | 1.075 | 20.000 | 0 | 0 | 6034 |
| `comms.link` | 6000 | 33078 / 516.8 | 2000 | 0 | 1.476 | 50.000 | 0 | 0 | 6034 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 22554 | 2910 / 45.5 |
| timer | 21942 | 3245 / 50.7 |
| intake | 1554 | 2488 / 38.9 |
| lock | 16554 | 456 / 7.1 |
| unlock | 16554 | 1853 / 29.0 |
| job_end | 22554 | 1979 / 30.9 |
| now | 24555 | 238 / 3.7 |
| dma | 83998 | 718 / 11.2 |
| epilogue | 22554 | 1319 / 20.6 |
| view | 22656 | 403 / 6.3 |
| send | 4727 | 2659 / 41.5 |
| recv | 25725 | 1161 / 18.1 |

Hold (L_kernel): 4468 cycles (69.8 µs) over 240716 holds.

Events: 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

Verdict of the run: **pass** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).
