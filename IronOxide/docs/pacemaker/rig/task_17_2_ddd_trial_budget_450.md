# Task 9.2: measurement run, set ddd-90

> A trial for task 17.2 (finding B36): the measurement declaration with the three sporadic Budgets at 450 µs for this run only. The table's Budget column is the declaration's of the board variant (350 µs). The declaration was put back afterwards.

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,set-env): sha256:5f256c35d2a376f59235f2dd6de6afb84701c1a896fc9fd1d19e8322df85d3e3, Generated_Config checksum 0x6db2ad66.
Scenario: nrf52840_measure, set ddd-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 751 | 19257 / 300.9 | 350 | 0 | 0.374 | 2.000 | 751 | 2150 | 0 |
| `therapy.sense_v` | 375 | 16703 / 261.0 | 350 | 0 | 0.336 | 3.000 | 375 | 2183 | 0 |
| `therapy.sense_a` | 375 | 16055 / 250.9 | 350 | 0 | 0.326 | 4.000 | 375 | 2172 | 0 |
| `therapy.control` | 15000 | 26688 / 417.0 | 5000 | 0 | 0.985 | 20.000 | 0 | 0 | 4790 |
| `comms.link` | 6000 | 29390 / 459.2 | 2000 | 0 | 1.386 | 50.000 | 0 | 0 | 3386 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 22501 | 2854 / 44.6 |
| timer | 21898 | 3125 / 48.8 |
| intake | 1501 | 2323 / 36.3 |
| lock | 16501 | 456 / 7.1 |
| unlock | 16501 | 1830 / 28.6 |
| job_end | 22501 | 1981 / 31.0 |
| now | 19770 | 238 / 3.7 |
| dma | 83998 | 719 / 11.2 |
| epilogue | 22501 | 442 / 6.9 |
| view | 22501 | 403 / 6.3 |
| send | 5398 | 2643 / 41.3 |
| recv | 26398 | 1146 / 17.9 |

Hold (L_kernel): 4348 cycles (67.9 µs) over 236966 holds.

Events: 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

Verdict of the run: **pass** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).
