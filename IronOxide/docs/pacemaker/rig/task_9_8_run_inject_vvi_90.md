# Task 9.8: injection run, set vvi-90

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,inject,set-vvi-90): sha256:5ae450cce9085352b5d0921858e41ee9c5104a5d66983e6e22e202751215a78d, Generated_Config checksum 0x6db2ad66.
Scenario: nrf52840_measure, set vvi-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 299 | 14397 / 225.0 | 450 | 0 | 0.297 | 2.000 | 299 | 2118 | 0 |
| `therapy.sense_v` | 375 | 20659 / 322.8 | 450 | 0 | 0.397 | 3.000 | 375 | 2148 | 0 |
| `therapy.sense_a` | 375 | 18363 / 286.9 | 450 | 0 | 0.362 | 4.000 | 375 | 2172 | 0 |
| `therapy.control` | 14850 | 35188 / 549.8 | 5000 | 0 | 1.071 | 20.000 | 0 | 0 | 5666 |
| `comms.link` | 6000 | 30142 / 471.0 | 2000 | 0 | 5.725 | 50.000 | 0 | 0 | 3386 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 22049 | 2854 / 44.6 |
| timer | 21597 | 6783 / 106.0 |
| intake | 1049 | 2315 / 36.2 |
| lock | 16049 | 456 / 7.1 |
| unlock | 16049 | 1819 / 28.4 |
| job_end | 21899 | 1978 / 30.9 |
| now | 87545 | 238 / 3.7 |
| dma | 83998 | 716 / 11.2 |
| fault | 75 | 5820 / 90.9 |
| epilogue | 22049 | 442 / 6.9 |
| restart | 150 | 6783 / 106.0 |
| enforce | 75 | 6783 / 106.0 |
| view | 22049 | 403 / 6.3 |
| send | 5531 | 2615 / 40.9 |
| recv | 26403 | 1124 / 17.6 |

Hold (L_kernel): 8513 cycles (133.0 µs) over 302243 holds.

Events: 2 events mirrored (2 held, 0 of them the post-detach reboot's): overrun of Task 3: 1, debugger of Task none: 1; the trace froze at event 1 (overrun of Task 3 at 1005.087 ms)

The trace's last 48 entries before it froze:

```
# 48 entries; t_ms tag value
   950.428 svc       0x000003 number=3
   950.468 epilogue  0x000028
   959.613 irq       0x000019 vector=25
   959.661 irq       0x000026 vector=38
   959.699 dispatch  0x00afe2 task=2 frame=0xafe0
   959.717 svc       0x000001 number=1
   959.862 svc       0x000002 number=2
   959.900 svc       0x000003 number=3
   959.939 epilogue  0x000026
   960.005 irq       0x000018 vector=24
   960.012 tick      0xea6099
   960.065 irq       0x000027 vector=39
   960.104 dispatch  0x00afe3 task=3 frame=0xafe0
   960.125 svc       0x000009 number=9
   960.148 svc       0x000001 number=1
   960.298 svc       0x000002 number=2
   960.374 svc       0x000003 number=3
   960.412 epilogue  0x000027
   980.005 irq       0x000018 vector=24
   980.012 tick      0xef4299
   980.065 irq       0x000027 vector=39
   980.104 dispatch  0x00afe3 task=3 frame=0xafe0
   980.125 svc       0x000009 number=9
   980.148 svc       0x000001 number=1
   980.298 svc       0x000002 number=2
   980.374 svc       0x000003 number=3
   980.412 epilogue  0x000027
   986.679 irq       0x00002a vector=42
   986.724 irq       0x000024 vector=36
   986.763 dispatch  0x00afe0 task=0 frame=0xafe0
   986.781 svc       0x000001 number=1
   986.915 svc       0x000002 number=2
   986.952 svc       0x000003 number=3
   986.989 epilogue  0x000024
  1000.005 irq       0x000018 vector=24
  1000.012 tick      0xf42499
  1000.072 irq       0x000027 vector=39
  1000.112 dispatch  0x00afe3 task=3 frame=0xafe0
  1000.133 svc       0x000009 number=9
  1000.154 irq       0x000018 vector=24
  1000.161 tick      0xf42de6
  1000.209 svc       0x000001 number=1
  1000.360 svc       0x000002 number=2
  1005.027 irq       0x000018 vector=24
  1005.033 tick      0xf55e71
  1005.082 irq       0x000018 vector=24
  1005.089 tick      0xf561e9
  1005.132 reinit    0x000000
```

Verdict of the run: **pass** (a fault, an enforcement and a restart entry each ran, and the cycle counter ran).
