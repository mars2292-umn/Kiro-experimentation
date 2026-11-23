# Task 9.8: injection run, set vvi-90

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,inject,set-vvi-90): sha256:849b58a13010fc15f762e16a1f7ad887003e51358ac11c6b54492b344d025a77, Generated_Config checksum 0xec5fe19d.
Scenario: nrf52840_measure, set vvi-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 448 | 16366 / 255.7 | 400 | 0 | 0.327 | 2.000 | 448 | 2028 | 0 |
| `therapy.sense_v` | 375 | 14739 / 230.3 | 400 | 0 | 0.302 | 3.000 | 375 | 2067 | 0 |
| `therapy.sense_a` | 375 | 9683 / 151.3 | 400 | 0 | 0.223 | 4.000 | 375 | 2042 | 0 |
| `therapy.control` | 14850 | 22182 / 346.6 | 5000 | 0 | 0.798 | 20.000 | 0 | 0 | 5797 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 16198 | 2748 / 42.9 |
| timer | 18746 | 5685 / 88.8 |
| intake | 1198 | 2208 / 34.5 |
| lock | 16198 | 439 / 6.9 |
| unlock | 16198 | 1690 / 26.4 |
| job_end | 16048 | 1826 / 28.5 |
| now | 88283 | 233 / 3.6 |
| fault | 75 | 4678 / 73.1 |
| epilogue | 16198 | 433 / 6.8 |
| restart | 150 | 5685 / 88.8 |
| enforce | 75 | 5685 / 88.8 |
| view | 16198 | 403 / 6.3 |

Hold (L_kernel): 7377 cycles (115.3 µs) over 172943 holds.

Events: 2 events mirrored (2 held, 0 of them the post-detach reboot's): overrun of Task 3: 1, debugger of Task none: 1; the trace froze at event 1 (overrun of Task 3 at 1005.080 ms)

Verdict of the run: **pass** (a fault, an enforcement and a restart entry each ran, and the cycle counter ran).
