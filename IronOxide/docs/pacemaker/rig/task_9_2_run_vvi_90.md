# Task 9.2: measurement run, set vvi-90

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,set-vvi-90): sha256:962cc2bb335802e269f16ab2e0f08d5218306160e3dccf9e922598b24a571ad1, Generated_Config checksum 0xec5fe19d.
Scenario: nrf52840_measure, set vvi-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 375 | 14562 / 227.5 | 400 | 0 | 0.296 | 2.000 | 375 | 2052 | 0 |
| `therapy.sense_v` | 375 | 14943 / 233.5 | 400 | 0 | 0.305 | 3.000 | 375 | 2067 | 0 |
| `therapy.sense_a` | 375 | 12919 / 201.9 | 400 | 0 | 0.274 | 4.000 | 375 | 2066 | 0 |
| `therapy.control` | 15000 | 15298 / 239.0 | 5000 | 0 | 0.767 | 20.000 | 0 | 0 | 4992 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 16125 | 2752 / 43.0 |
| timer | 18522 | 2804 / 43.8 |
| intake | 1125 | 2208 / 34.5 |
| lock | 16125 | 438 / 6.8 |
| unlock | 16125 | 1690 / 26.4 |
| job_end | 16125 | 1756 / 27.4 |
| now | 31004 | 233 / 3.6 |
| epilogue | 16125 | 434 / 6.8 |
| view | 16125 | 403 / 6.3 |

Hold (L_kernel): 3560 cycles (55.6 µs) over 115150 holds.

Events: 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

Verdict of the run: **pass** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).
