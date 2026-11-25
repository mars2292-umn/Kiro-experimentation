# Task 9.2: measurement run, set ddd-90

> Kept for task 17.2 (finding B36): the third run of the DDD set with the declared Budgets of 350 µs, in which a Job of `sense_v` reached its Budget at 55.1 s. The trace at the end shows the Job (dispatch of task 1) with two of the Kernel's timer entries (`tick`) inside it before the third, which is the Budget's.

Binary `pacemaker` (nrf52840,debug-port,quiet,trace-ram,measure,set-env): sha256:0024ab63d1d5c08cb07d430748f09fac3848961efd9f42ceeeb5a7833059710f, Generated_Config checksum 0x9b1788eb.
Scenario: nrf52840_measure, set ddd-90, Heart_Simulator `fixed` (300 s).

| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |
|---|---|---|---|---|---|---|---|---|---|
| `therapy.pace` | 750 | 18977 / 296.5 | 350 | 0 | 0.373 | 2.000 | 750 | 2150 | 0 |
| `therapy.sense_v` | 374 | 16703 / 261.0 | 350 | 0 | 0.336 | 3.000 | 375 | 2164 | 0 |
| `therapy.sense_a` | 375 | 15895 / 248.4 | 350 | 0 | 0.323 | 4.000 | 375 | 2168 | 0 |
| `therapy.control` | 15000 | 26688 / 417.0 | 5000 | 0 | 0.985 | 20.000 | 0 | 0 | 5438 |
| `comms.link` | 6000 | 29438 / 460.0 | 2000 | 0 | 1.386 | 50.000 | 0 | 0 | 3386 |

Kernel entries (Rust halves):

| Entry | Runs | Max (cycles / µs) |
|---|---|---|
| dispatch | 22500 | 2854 / 44.6 |
| timer | 21898 | 6455 / 100.9 |
| intake | 1500 | 2311 / 36.1 |
| lock | 16500 | 456 / 7.1 |
| unlock | 16500 | 1822 / 28.5 |
| job_end | 22499 | 1981 / 31.0 |
| now | 19891 | 238 / 3.7 |
| dma | 83998 | 719 / 11.2 |
| epilogue | 22500 | 442 / 6.9 |
| restart | 1 | 6455 / 100.9 |
| enforce | 1 | 6455 / 100.9 |
| view | 22500 | 403 / 6.3 |
| send | 5399 | 2643 / 41.3 |
| recv | 26398 | 1146 / 17.9 |

Hold (L_kernel): 8164 cycles (127.6 µs) over 237082 holds.

Events: 2 events mirrored (2 held, 0 of them the post-detach reboot's): overrun of Task 1: 1, debugger of Task none: 1; the trace froze at event 1 (overrun of Task 1 at 55129.792 ms)

The trace's last 48 entries before it froze:

```
# 48 entries; t_ms tag value
 55100.115 dispatch  0x00afe3 task=3 frame=0xafe0
 55100.135 svc       0x000009 number=9
 55100.159 svc       0x000001 number=1
 55100.269 svc       0x000002 number=2
 55100.314 svc       0x000003 number=3
 55100.356 epilogue  0x000027
 55100.372 irq       0x000028 vector=40
 55100.410 dispatch  0x0107e4 task=4 frame=0x107e0
 55100.428 svc       0x000006 number=6
 55100.448 svc       0x000006 number=6
 55100.466 svc       0x000006 number=6
 55100.484 svc       0x000006 number=6
 55100.502 svc       0x000006 number=6
 55100.522 svc       0x000006 number=6
 55100.541 svc       0x000006 number=6
 55100.559 svc       0x000006 number=6
 55100.577 svc       0x000006 number=6
 55100.609 svc       0x000009 number=9
 55100.650 svc       0x000008 number=8
 55100.709 svc       0x000006 number=6
 55100.727 svc       0x000006 number=6
 55100.749 svc       0x000006 number=6
 55100.772 svc       0x000006 number=6
 55100.790 svc       0x000006 number=6
 55100.809 svc       0x000003 number=3
 55100.849 epilogue  0x000028
 55120.005 irq       0x000018 vector=24
 55120.012 tick      0x910899
 55120.066 irq       0x000027 vector=39
 55120.105 dispatch  0x00afe3 task=3 frame=0xafe0
 55120.126 svc       0x000009 number=9
 55120.171 svc       0x000009 number=9
 55120.194 svc       0x000001 number=1
 55120.305 svc       0x000002 number=2
 55120.349 svc       0x000003 number=3
 55120.388 epilogue  0x000027
 55129.370 irq       0x00001a vector=26
 55129.417 irq       0x000025 vector=37
 55129.456 dispatch  0x00afe1 task=1 frame=0xafe0
 55129.474 svc       0x000001 number=1
 55129.509 irq       0x000018 vector=24
 55129.516 tick      0x935a9d
 55129.697 svc       0x000002 number=2
 55129.733 irq       0x000018 vector=24
 55129.740 tick      0x936898
 55129.788 irq       0x000018 vector=24
 55129.794 tick      0x936c01
 55129.836 reinit    0x000000
```

Verdict of the run: **FAIL** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).
