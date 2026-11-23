# Task 9.8: injection run, set vvi-90

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,inject,set-vvi-90): sha256:1b902900f8f7e8a281a1515a7494ac5741d9b52aad69994739e7704115efab94, Generated_Config checksum 0x2d3eb664.
Scenario: nrf52840_measure, set vvi-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 299 | 11366 / 177.6 | 350 | 0 | 0.247 | 2.000 | 299 | 2024 | 0 |
| `therapy.sense_v` | 375 | 11623 / 181.6 | 350 | 0 | 0.300 | 3.000 | 375 | 2043 | 0 |
| `therapy.sense_a` | 375 | 12799 / 200.0 | 350 | 0 | 0.271 | 4.000 | 375 | 2066 | 0 |
| `therapy.control` | 14850 | 20362 / 318.2 | 5000 | 0 | 0.798 | 20.000 | 0 | 0 | 4636 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 16049 | 2748 / 42.9 |
| timer | 18597 | 5565 / 87.0 |
| intake | 1049 | 2208 / 34.5 |
| lock | 16049 | 439 / 6.9 |
| unlock | 16049 | 1690 / 26.4 |
| job_end | 15899 | 1754 / 27.4 |
| now | 88003 | 233 / 3.6 |
| fault | 75 | 4678 / 73.1 |
| epilogue | 16049 | 433 / 6.8 |
| restart | 150 | 5565 / 87.0 |
| enforce | 75 | 5565 / 87.0 |
| view | 16049 | 403 / 6.3 |

Hold (L_kernel): 7245 cycles (113.2 µs) over 171769 holds.

Events: 2 events mirrored (2 held, 0 of them the post-detach reboot's): overrun of Task 3: 1, debugger of Task none: 1; the trace froze at event 1 (overrun of Task 3 at 1005.081 ms)

Verdict of the run: **pass** (a fault, an enforcement and a restart entry each ran, and the cycle counter ran).
