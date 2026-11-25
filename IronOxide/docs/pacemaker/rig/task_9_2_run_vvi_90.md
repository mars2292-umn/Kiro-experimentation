# Task 9.2: measurement run, set vvi-90

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,set-vvi-90): sha256:de2b22620d0164afa4d69fa2a21ae35840e28abdd359935c071705833efec079, Generated_Config checksum 0x6db2ad66.
Scenario: nrf52840_measure, set vvi-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 375 | 16529 / 258.3 | 450 | 0 | 0.332 | 2.000 | 375 | 2142 | 0 |
| `therapy.sense_v` | 375 | 16463 / 257.2 | 450 | 0 | 0.348 | 3.000 | 375 | 2164 | 0 |
| `therapy.sense_a` | 375 | 17443 / 272.5 | 450 | 0 | 0.348 | 4.000 | 375 | 2168 | 0 |
| `therapy.control` | 15000 | 29244 / 456.9 | 5000 | 0 | 1.025 | 20.000 | 0 | 0 | 4378 |
| `comms.link` | 6000 | 29442 / 460.0 | 2000 | 0 | 1.426 | 50.000 | 0 | 0 | 3386 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 22125 | 2838 / 44.3 |
| timer | 21521 | 3125 / 48.8 |
| intake | 1125 | 2311 / 36.1 |
| lock | 16125 | 456 / 7.1 |
| unlock | 16125 | 1819 / 28.4 |
| job_end | 22125 | 1981 / 31.0 |
| now | 31759 | 238 / 3.7 |
| dma | 83998 | 718 / 11.2 |
| epilogue | 22125 | 442 / 6.9 |
| view | 22125 | 403 / 6.3 |
| send | 5397 | 2658 / 41.5 |
| recv | 26397 | 1161 / 18.1 |

Hold (L_kernel): 4348 cycles (67.9 µs) over 246696 holds.

Events: 3 events mirrored (3 held, 0 of them the post-detach reboot's): operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

Verdict of the run: **pass** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).
