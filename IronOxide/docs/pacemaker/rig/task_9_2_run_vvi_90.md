# Task 9.2: measurement run, set vvi-90

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,set-vvi-90): sha256:fc2c4d4f3fa2afb84ec314937bf2b871d4facb8279b6cb9ad8fa88e90a56e76f, Generated_Config checksum 0x2d3eb664.
Scenario: nrf52840_measure, set vvi-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 374 | 10898 / 170.3 | 350 | 0 | 0.241 | 2.000 | 374 | 2052 | 0 |
| `therapy.sense_v` | 375 | 11079 / 173.1 | 350 | 0 | 0.245 | 3.000 | 375 | 2067 | 0 |
| `therapy.sense_a` | 375 | 8979 / 140.3 | 350 | 0 | 0.212 | 4.000 | 375 | 2066 | 0 |
| `therapy.control` | 15000 | 15102 / 236.0 | 5000 | 0 | 0.764 | 20.000 | 0 | 0 | 3636 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 16124 | 2752 / 43.0 |
| timer | 18520 | 2804 / 43.8 |
| intake | 1124 | 2208 / 34.5 |
| lock | 16124 | 439 / 6.9 |
| unlock | 16124 | 1688 / 26.4 |
| job_end | 16124 | 1756 / 27.4 |
| now | 31002 | 233 / 3.6 |
| epilogue | 16124 | 433 / 6.8 |
| view | 16124 | 403 / 6.3 |

Hold (L_kernel): 3560 cycles (55.6 µs) over 115141 holds.

Events: 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

Verdict of the run: **pass** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).
