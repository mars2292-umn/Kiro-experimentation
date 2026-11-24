# Task 9.2: measurement run, set ddd-90

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,set-env): sha256:345905f1bb9c4bda6d2261b5858917f72413a1248565deac1c85eadffa273cc6, Generated_Config checksum 0x2d3eb664.
Scenario: nrf52840_measure, set ddd-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 750 | 15882 / 248.2 | 350 | 0 | 0.334 | 2.000 | 750 | 2255 | 0 |
| `therapy.sense_v` | 375 | 16259 / 254.0 | 350 | 0 | 0.326 | 3.000 | 375 | 2067 | 0 |
| `therapy.sense_a` | 375 | 12263 / 191.6 | 350 | 0 | 0.263 | 4.000 | 375 | 2066 | 0 |
| `therapy.control` | 15000 | 16094 / 251.5 | 5000 | 0 | 0.780 | 20.000 | 0 | 0 | 6325 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 16500 | 2840 / 44.4 |
| timer | 18895 | 2844 / 44.4 |
| intake | 1500 | 2355 / 36.8 |
| lock | 16500 | 439 / 6.9 |
| unlock | 16500 | 1791 / 28.0 |
| job_end | 16500 | 1875 / 29.3 |
| now | 19872 | 233 / 3.6 |
| epilogue | 16500 | 1329 / 20.8 |
| view | 16520 | 403 / 6.3 |

Hold (L_kernel): 4182 cycles (65.3 µs) over 106266 holds.

Events: 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

Verdict of the run: **pass** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).
