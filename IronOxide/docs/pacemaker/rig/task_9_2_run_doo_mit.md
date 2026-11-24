# Task 9.2: measurement run, set doo-mit

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,set-env): sha256:87f06192b74886b7138b8ad2b7aea63300d18a26ff7b4bc4d1d885020682902a, Generated_Config checksum 0x2d3eb664.
Scenario: nrf52840_measure, set doo-mit, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 1749 | 12758 / 199.3 | 350 | 0 | 0.270 | 2.000 | 1749 | 2056 | 0 |
| `therapy.sense_v` | 375 | 12011 / 187.7 | 350 | 0 | 0.257 | 3.000 | 375 | 2067 | 0 |
| `therapy.sense_a` | 375 | 12191 / 190.5 | 350 | 0 | 0.262 | 4.000 | 375 | 2066 | 0 |
| `therapy.control` | 15000 | 16094 / 251.5 | 5000 | 0 | 0.780 | 20.000 | 0 | 0 | 5160 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 17499 | 2748 / 42.9 |
| timer | 19897 | 2804 / 43.8 |
| intake | 2499 | 2208 / 34.5 |
| lock | 17499 | 439 / 6.9 |
| unlock | 17499 | 1691 / 26.4 |
| job_end | 17499 | 1756 / 27.4 |
| now | 21501 | 232 / 3.6 |
| epilogue | 17499 | 434 / 6.8 |
| view | 17499 | 403 / 6.3 |

Hold (L_kernel): 3560 cycles (55.6 µs) over 113892 holds.

Events: 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

Verdict of the run: **pass** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).
