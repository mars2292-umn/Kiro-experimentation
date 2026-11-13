# HIL boot-check suite: mismatched values and corrupted tables (task 7.2, R12.6)

Verdict: **pass**

Target 1366:1060:001050217737; binary sha256:794519a67d044355242d158a0ecf3b0a3c02187c54be1e2b1d0308b392fc4ce2 (the HIL build with the `boot-mutation` feature; one run per case, the mutation word written over the probe before each).

## Cases

| Case | Result | Detail |
|---|---|---|
| case 1 (R12.1): the recorded CPUID differs from the Target's (ASM-01) | pass | safe state reached; boot_check detail 0x100; bit 8 recorded alone; no Task released |
| case 2 (R12.1): the recorded priority bits (8) differ from the implemented 3 (ASM-03) | pass | safe state reached; boot_check detail 0x10; bit 4 recorded alone; no Task released |
| case 3 (R12.1): the recorded clock source (the RC oscillator) differs from the running crystal (ASM-05) | pass | safe state reached; boot_check detail 0x400; bit 10 recorded alone; no Task released |
| case 4 (R12.1): PRIGROUP changed from its reset value before the check (ASM-03) | pass | safe state reached; boot_check detail 0x200; bit 9 recorded alone; no Task released |
| case 5 (R12.1): the instruction cache enabled before the check (ASM-04) | pass | safe state reached; boot_check detail 0x800; bit 11 recorded alone; no Task released |
| case 6 (R12.3): the recorded CONFIG checksum differs from the tables' | pass | safe state reached; boot_check detail 0x20; bit 5 recorded alone; no Task released |
| case 7 (R12.3): the CONFIG table corrupted in memory (a Task's Budget changed) | pass | safe state reached; boot_check detail 0x20; bit 5 recorded alone; no Task released |
| case 8 (R12.3): the recorded LAYOUT checksum differs from the tables' | pass | safe state reached; boot_check detail 0x2000; bit 13 recorded alone; no Task released |
| case 9 (R12.2): the recorded interrupt-line count (600) exceeds what the Kernel can serve | pass | safe state reached; boot_check detail 0x40; bit 6 recorded alone; no Task released |
| case 10 (R12.2): the watchdog started with a 50 ms reload and never fed: the reset lands inside the run and the next boot fails the watchdog check (ORQ-31) | pass | safe state reached; boot_check detail 0x80; bit 7 recorded alone; no Task released |
| no mutation: the build boots and runs (RSK-OK) | pass | RSK-OK after 10.8 s |

## Target consoles

### mutation 1

```

RSK-SAFE-STATE
control=0
sensor=0
log=0
pulse=0
echo=0
blocked_releases=0
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=3
event=23
  task=255
  at_ms=0
  detail=1
  prs_lo=0
  prs_hi=0
event=14
  task=255
  at_ms=0
  detail=256
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=0
  detail=0
  prs_lo=0
  prs_hi=0
RSK-FAIL
Firmware exited with: Application exit (Unknown exit code 1)
Core 0
    Frame 0: semihosting_exit @ 0x548
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs:1472:17
    Frame 1: report_and_exit @ 0xf3d4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/system/src/lib.rs:527:5
    Frame 2: safe_state @ 0xf8c4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/decl/src/nrf52840.rs:9:1
    Frame 3: Reset @ 0x1fcc
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-entry/src/lib.rs:142:9
    Frame 4: Reset @ 0x0
     Finished in 8.05s
Error: Application exit
```

### mutation 2

```

RSK-SAFE-STATE
control=0
sensor=0
log=0
pulse=0
echo=0
blocked_releases=0
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=3
event=23
  task=255
  at_ms=0
  detail=1
  prs_lo=0
  prs_hi=0
event=14
  task=255
  at_ms=0
  detail=16
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=0
  detail=0
  prs_lo=0
  prs_hi=0
RSK-FAIL
Firmware exited with: Application exit (Unknown exit code 1)
Core 0
    Frame 0: semihosting_exit @ 0x548
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs:1472:17
    Frame 1: report_and_exit @ 0xf3d4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/system/src/lib.rs:527:5
    Frame 2: safe_state @ 0xf8c4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/decl/src/nrf52840.rs:9:1
    Frame 3: Reset @ 0x1fcc
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-entry/src/lib.rs:142:9
    Frame 4: Reset @ 0x0
     Finished in 8.04s
Error: Application exit
```

### mutation 3

```

RSK-SAFE-STATE
control=0
sensor=0
log=0
pulse=0
echo=0
blocked_releases=0
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=3
event=23
  task=255
  at_ms=0
  detail=1
  prs_lo=0
  prs_hi=0
event=14
  task=255
  at_ms=0
  detail=1024
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=0
  detail=0
  prs_lo=0
  prs_hi=0
RSK-FAIL
Firmware exited with: Application exit (Unknown exit code 1)
Core 0
    Frame 0: semihosting_exit @ 0x548
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs:1472:17
    Frame 1: report_and_exit @ 0xf3d4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/system/src/lib.rs:527:5
    Frame 2: safe_state @ 0xf8c4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/decl/src/nrf52840.rs:9:1
    Frame 3: Reset @ 0x1fcc
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-entry/src/lib.rs:142:9
    Frame 4: Reset @ 0x0
     Finished in 8.06s
Error: Application exit
```

### mutation 4

```

RSK-SAFE-STATE
control=0
sensor=0
log=0
pulse=0
echo=0
blocked_releases=0
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=3
event=23
  task=255
  at_ms=0
  detail=1
  prs_lo=0
  prs_hi=0
event=14
  task=255
  at_ms=0
  detail=512
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=0
  detail=0
  prs_lo=0
  prs_hi=0
RSK-FAIL
Firmware exited with: Application exit (Unknown exit code 1)
Core 0
    Frame 0: semihosting_exit @ 0x548
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs:1472:17
    Frame 1: report_and_exit @ 0xf3d4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/system/src/lib.rs:527:5
    Frame 2: safe_state @ 0xf8c4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/decl/src/nrf52840.rs:9:1
    Frame 3: Reset @ 0x1fcc
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-entry/src/lib.rs:142:9
    Frame 4: Reset @ 0x0
     Finished in 8.09s
Error: Application exit
```

### mutation 5

```

RSK-SAFE-STATE
control=0
sensor=0
log=0
pulse=0
echo=0
blocked_releases=0
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=3
event=23
  task=255
  at_ms=0
  detail=1
  prs_lo=0
  prs_hi=0
event=14
  task=255
  at_ms=0
  detail=2048
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=0
  detail=0
  prs_lo=0
  prs_hi=0
RSK-FAIL
Firmware exited with: Application exit (Unknown exit code 1)
Core 0
    Frame 0: semihosting_exit @ 0x548
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs:1472:17
    Frame 1: report_and_exit @ 0xf3d4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/system/src/lib.rs:527:5
    Frame 2: safe_state @ 0xf8c4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/decl/src/nrf52840.rs:9:1
    Frame 3: Reset @ 0x1fcc
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-entry/src/lib.rs:142:9
    Frame 4: Reset @ 0x0
     Finished in 8.00s
Error: Application exit
```

### mutation 6

```

RSK-SAFE-STATE
control=0
sensor=0
log=0
pulse=0
echo=0
blocked_releases=0
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=3
event=23
  task=255
  at_ms=0
  detail=1
  prs_lo=0
  prs_hi=0
event=14
  task=255
  at_ms=0
  detail=32
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=0
  detail=0
  prs_lo=0
  prs_hi=0
RSK-FAIL
Firmware exited with: Application exit (Unknown exit code 1)
Core 0
    Frame 0: semihosting_exit @ 0x548
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs:1472:17
    Frame 1: report_and_exit @ 0xf3d4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/system/src/lib.rs:527:5
    Frame 2: safe_state @ 0xf8c4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/decl/src/nrf52840.rs:9:1
    Frame 3: Reset @ 0x1fcc
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-entry/src/lib.rs:142:9
    Frame 4: Reset @ 0x0
     Finished in 8.04s
Error: Application exit
```

### mutation 7

```

RSK-SAFE-STATE
control=0
sensor=0
log=0
pulse=0
echo=0
blocked_releases=0
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=3
event=23
  task=255
  at_ms=0
  detail=1
  prs_lo=0
  prs_hi=0
event=14
  task=255
  at_ms=0
  detail=32
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=0
  detail=0
  prs_lo=0
  prs_hi=0
RSK-FAIL
Firmware exited with: Application exit (Unknown exit code 1)
Core 0
    Frame 0: semihosting_exit @ 0x548
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs:1472:17
    Frame 1: report_and_exit @ 0xf3d4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/system/src/lib.rs:527:5
    Frame 2: safe_state @ 0xf8c4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/decl/src/nrf52840.rs:9:1
    Frame 3: Reset @ 0x1fcc
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-entry/src/lib.rs:142:9
    Frame 4: Reset @ 0x0
     Finished in 8.03s
Error: Application exit
```

### mutation 8

```

RSK-SAFE-STATE
control=0
sensor=0
log=0
pulse=0
echo=0
blocked_releases=0
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=3
event=23
  task=255
  at_ms=0
  detail=1
  prs_lo=0
  prs_hi=0
event=14
  task=255
  at_ms=0
  detail=8192
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=0
  detail=0
  prs_lo=0
  prs_hi=0
RSK-FAIL
Firmware exited with: Application exit (Unknown exit code 1)
Core 0
    Frame 0: semihosting_exit @ 0x548
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs:1472:17
    Frame 1: report_and_exit @ 0xf3d4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/system/src/lib.rs:527:5
    Frame 2: safe_state @ 0xf8c4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/decl/src/nrf52840.rs:9:1
    Frame 3: Reset @ 0x1fcc
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-entry/src/lib.rs:142:9
    Frame 4: Reset @ 0x0
     Finished in 8.07s
Error: Application exit
```

### mutation 9

```

RSK-SAFE-STATE
control=0
sensor=0
log=0
pulse=0
echo=0
blocked_releases=0
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=3
event=23
  task=255
  at_ms=0
  detail=1
  prs_lo=0
  prs_hi=0
event=14
  task=255
  at_ms=0
  detail=64
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=0
  detail=0
  prs_lo=0
  prs_hi=0
RSK-FAIL
Firmware exited with: Application exit (Unknown exit code 1)
Core 0
    Frame 0: semihosting_exit @ 0x548
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs:1472:17
    Frame 1: report_and_exit @ 0xf3d4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/system/src/lib.rs:527:5
    Frame 2: safe_state @ 0xf8c4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/decl/src/nrf52840.rs:9:1
    Frame 3: Reset @ 0x1fcc
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-entry/src/lib.rs:142:9
    Frame 4: Reset @ 0x0
     Finished in 8.05s
Error: Application exit
```

### mutation 10

```

RSK-SAFE-STATE
control=0
sensor=0
log=0
pulse=0
echo=0
blocked_releases=0
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=4
event=23
  task=255
  at_ms=0
  detail=1
  prs_lo=0
  prs_hi=0
event=15
  task=255
  at_ms=0
  detail=1
  prs_lo=0
  prs_hi=0
event=14
  task=255
  at_ms=0
  detail=128
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=0
  detail=0
  prs_lo=0
  prs_hi=0
RSK-FAIL
Firmware exited with: Application exit (Unknown exit code 1)
Core 0
    Frame 0: semihosting_exit @ 0x548
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs:1472:17
    Frame 1: report_and_exit @ 0xf3d4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/system/src/lib.rs:527:5
    Frame 2: safe_state @ 0xf8c4
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/apps/p1-demo/decl/src/nrf52840.rs:9:1
    Frame 3: Reset @ 0x1fcc
        /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/crates/rsk-entry/src/lib.rs:142:9
    Frame 4: Reset @ 0x0
     Finished in 8.04s
Error: Application exit
```

### mutation 0

```

RSK-SAFE-STATE
control=20
sensor=9
log=5
pulse=10
echo=6
blocked_releases=9
filter=8
filter_state_bits=1115899705
sampler=2
dma_ok=2
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=3
event=23
  task=255
  at_ms=0
  detail=1
  prs_lo=0
  prs_hi=0
event=10
  task=255
  at_ms=200
  detail=0
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=200
  detail=0
  prs_lo=0
  prs_hi=0
RSK-OK
Firmware exited successfully
     Finished in 7.93s
```

## Commands

    probe-rs write --chip nRF52840_xxAA --probe 1366:1060:001050217737 b32 0x2003fff0 0x1
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/target/xtask-hil/boot-checks/thumbv7em-none-eabihf/release/p1-demo
    probe-rs write --chip nRF52840_xxAA --probe 1366:1060:001050217737 b32 0x2003fff0 0x2
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/target/xtask-hil/boot-checks/thumbv7em-none-eabihf/release/p1-demo
    probe-rs write --chip nRF52840_xxAA --probe 1366:1060:001050217737 b32 0x2003fff0 0x3
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/target/xtask-hil/boot-checks/thumbv7em-none-eabihf/release/p1-demo
    probe-rs write --chip nRF52840_xxAA --probe 1366:1060:001050217737 b32 0x2003fff0 0x4
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/target/xtask-hil/boot-checks/thumbv7em-none-eabihf/release/p1-demo
    probe-rs write --chip nRF52840_xxAA --probe 1366:1060:001050217737 b32 0x2003fff0 0x5
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/target/xtask-hil/boot-checks/thumbv7em-none-eabihf/release/p1-demo
    probe-rs write --chip nRF52840_xxAA --probe 1366:1060:001050217737 b32 0x2003fff0 0x6
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/target/xtask-hil/boot-checks/thumbv7em-none-eabihf/release/p1-demo
    probe-rs write --chip nRF52840_xxAA --probe 1366:1060:001050217737 b32 0x2003fff0 0x7
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/target/xtask-hil/boot-checks/thumbv7em-none-eabihf/release/p1-demo
    probe-rs write --chip nRF52840_xxAA --probe 1366:1060:001050217737 b32 0x2003fff0 0x8
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/target/xtask-hil/boot-checks/thumbv7em-none-eabihf/release/p1-demo
    probe-rs write --chip nRF52840_xxAA --probe 1366:1060:001050217737 b32 0x2003fff0 0x9
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/target/xtask-hil/boot-checks/thumbv7em-none-eabihf/release/p1-demo
    probe-rs write --chip nRF52840_xxAA --probe 1366:1060:001050217737 b32 0x2003fff0 0xa
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault --no-catch-reset /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/target/xtask-hil/boot-checks/thumbv7em-none-eabihf/release/p1-demo
    probe-rs write --chip nRF52840_xxAA --probe 1366:1060:001050217737 b32 0x2003fff0 0x0
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation-fixes/IronOxide/target/xtask-hil/boot-checks/thumbv7em-none-eabihf/release/p1-demo
