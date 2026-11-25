# Task 9.8: injection run, set vvi-90

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,inject,set-vvi-90): sha256:81d2235f2a930e43c2d56178994712a6813617916fe58ea23ba08eaaa8f0a0d9, Generated_Config checksum 0x6db2ad66.
Scenario: nrf52840_measure, set vvi-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 299 | 16305 / 254.8 | 450 | 0 | 0.328 | 2.000 | 299 | 2118 | 0 |
| `therapy.sense_v` | 375 | 17479 / 273.1 | 450 | 0 | 0.348 | 3.000 | 375 | 2140 | 0 |
| `therapy.sense_a` | 375 | 17239 / 269.4 | 450 | 0 | 0.344 | 4.000 | 375 | 2168 | 0 |
| `therapy.control` | 14850 | 34272 / 535.5 | 5000 | 0 | 1.058 | 20.000 | 0 | 0 | 4270 |
| `comms.link` | 6000 | 30266 / 472.9 | 2000 | 0 | 5.730 | 50.000 | 0 | 0 | 3386 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 22049 | 2858 / 44.7 |
| timer | 21594 | 6971 / 108.9 |
| intake | 1049 | 2311 / 36.1 |
| lock | 16049 | 456 / 7.1 |
| unlock | 16049 | 1820 / 28.4 |
| job_end | 21899 | 1980 / 30.9 |
| now | 87620 | 238 / 3.7 |
| dma | 83998 | 718 / 11.2 |
| fault | 75 | 5880 / 91.9 |
| epilogue | 22049 | 442 / 6.9 |
| restart | 150 | 6971 / 108.9 |
| enforce | 75 | 6971 / 108.9 |
| view | 22049 | 403 / 6.3 |
| send | 5530 | 2631 / 41.1 |
| recv | 26402 | 1134 / 17.7 |

Hold (L_kernel): 8724 cycles (136.3 µs) over 302313 holds.

Events: 2 events mirrored (2 held, 0 of them the post-detach reboot's): overrun of Task 3: 1, debugger of Task none: 1; the trace froze at event 1 (overrun of Task 3 at 1005.096 ms)

The trace's last 48 entries before it froze:

```
# 48 entries; t_ms tag value
   950.141 svc       0x000006 number=6
   950.160 svc       0x000006 number=6
   950.178 svc       0x000006 number=6
   950.196 svc       0x000006 number=6
   950.216 svc       0x000006 number=6
   950.234 svc       0x000006 number=6
   950.253 svc       0x000006 number=6
   950.271 svc       0x000006 number=6
   950.296 svc       0x000009 number=9
   950.328 svc       0x000006 number=6
   950.346 svc       0x000006 number=6
   950.368 svc       0x000006 number=6
   950.391 svc       0x000006 number=6
   950.409 svc       0x000006 number=6
   950.429 svc       0x000003 number=3
   950.468 epilogue  0x000028
   960.005 irq       0x000018 vector=24
   960.012 tick      0xea6099
   960.066 irq       0x000027 vector=39
   960.105 dispatch  0x00afe3 task=3 frame=0xafe0
   960.126 svc       0x000009 number=9
   960.149 svc       0x000001 number=1
   960.285 svc       0x000002 number=2
   960.361 svc       0x000003 number=3
   960.401 epilogue  0x000027
   980.005 irq       0x000018 vector=24
   980.012 tick      0xef4299
   980.066 irq       0x000027 vector=39
   980.105 dispatch  0x00afe3 task=3 frame=0xafe0
   980.126 svc       0x000009 number=9
   980.149 svc       0x000001 number=1
   980.285 svc       0x000002 number=2
   980.361 svc       0x000003 number=3
   980.401 epilogue  0x000027
  1000.005 irq       0x000018 vector=24
  1000.012 tick      0xf42499
  1000.075 irq       0x000027 vector=39
  1000.115 dispatch  0x00afe3 task=3 frame=0xafe0
  1000.136 svc       0x000009 number=9
  1000.157 irq       0x000018 vector=24
  1000.163 tick      0xf42e0f
  1000.214 svc       0x000001 number=1
  1000.350 svc       0x000002 number=2
  1005.020 irq       0x000018 vector=24
  1005.027 tick      0xf55e03
  1005.091 irq       0x000018 vector=24
  1005.098 tick      0xf5627c
  1005.143 reinit    0x000000
```

Verdict of the run: **pass** (a fault, an enforcement and a restart entry each ran, and the cycle counter ran).
