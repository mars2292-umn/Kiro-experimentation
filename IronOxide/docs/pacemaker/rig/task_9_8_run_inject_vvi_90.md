# Task 9.8: injection run, set vvi-90

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,inject,set-vvi-90): sha256:12fa5dd2fadc22cf9de1b9e0d526db4c52d175189c438bd02b29bbb2f1c4e353, Generated_Config checksum 0x2d3eb664.
Scenario: nrf52840_measure, set vvi-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 300 | 10814 / 169.0 | 350 | 0 | 0.239 | 2.000 | 300 | 2024 | 0 |
| `therapy.sense_v` | 375 | 14191 / 221.7 | 350 | 0 | 0.293 | 3.000 | 375 | 2067 | 0 |
| `therapy.sense_a` | 375 | 8975 / 140.2 | 350 | 0 | 0.212 | 4.000 | 375 | 2042 | 0 |
| `therapy.control` | 14850 | 21982 / 343.5 | 5000 | 0 | 0.795 | 20.000 | 0 | 0 | 5020 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 16050 | 2748 / 42.9 |
| timer | 18599 | 5685 / 88.8 |
| intake | 1050 | 2208 / 34.5 |
| lock | 16050 | 439 / 6.9 |
| unlock | 16050 | 1690 / 26.4 |
| job_end | 15900 | 1753 / 27.4 |
| now | 87914 | 233 / 3.6 |
| fault | 75 | 4678 / 73.1 |
| epilogue | 16050 | 433 / 6.8 |
| restart | 150 | 5685 / 88.8 |
| enforce | 75 | 5685 / 88.8 |
| view | 16050 | 403 / 6.3 |

Hold (L_kernel): 7370 cycles (115.2 µs) over 171687 holds.

Events: 2 events mirrored (2 held, 0 of them the post-detach reboot's): overrun of Task 3: 1, debugger of Task none: 1; the trace froze at event 1 (overrun of Task 3 at 1005.074 ms)

Verdict of the run: **pass** (a fault, an enforcement and a restart entry each ran, and the cycle counter ran).
