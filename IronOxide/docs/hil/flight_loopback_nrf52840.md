# HIL_Rig self-test

Verdict: **pass**

Tools: probe-rs 0.32.0 (git commit: crates.io); sigrok-cli 0.7.2

## Rig

| Role | Probe | Board | Binary | Wired |
|---|---|---|---|---|
| target | `1366:1060:001050217737` | PCA10056 v3.0.3, nRF52840 AAF0 | (the Application under test) | yes |
| stimulus | `1366:1061:001050279833` | PCA10056 v3.0.3, nRF52840 AAF0 | hil-stimulus | yes |
| spare | `1366:1061:001050211852` | PCA10056 v3.0.3, nRF52840 AAF0 | (the Application under test) | no |

Analyzer: fx2lafw at 2000000 Hz; stimulus mirror on D6, response on D0.

Wiring:

- `board_spares`: P1.02 (D1), P1.03 (D2), P1.04 (D3)
- `ground`: common ground across both boards and the analyzer
- `response_line`: target P1.10 (D8) -> analyzer D0, toggled by every `pulse` Job
- `stimulus_line`: P1.01 (D0): stimulus -> target, the GPIOTE input of the target's `pulse` Task
- `stimulus_mirror`: stimulus P1.05 (D4) -> analyzer D6, driven in the same register write as the stimulus line
- `stimulus_spare`: stimulus P1.06 (D5) -> analyzer D7
- `target_spares`: P1.11 (D9) -> D1, P1.12 (D10) -> D2, P1.13 (D11) -> D3, P1.14 (D12) -> D4, P1.15 (D13) -> D5

## Steps

| Step | Result | Detail |
|---|---|---|
| boards by serial | pass | target = 1366:1060:001050217737 (J-Link); stimulus = 1366:1061:001050279833 (J-Link); spare = 1366:1061:001050211852 (J-Link, not wired) |
| analyzer | pass | fx2lafw:conn=0.5 - Saleae Logic [S/N: Saleae Logic] with 8 channels: D0 D1 D2 D3 D4 D5 D6 D7; 2000000 Hz; stimulus D6, response D0 |
| stimulus flashed | pass | /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus in 5.2 s |
| stimulus read back | pass | 8 parts, 47064 bytes, read-back hash sha256:a087c2d8a0a24cd580f25ca3aa80a505ec0f12fd3cea61a025e0178be085adc1, equal to the Build_Manifest's |
| stimulus running | pass | reset; the stimulus Application runs until the rig resets it again |
| target run | pass | RSK-OK in 9.4 s; pulse Releases counted by the target: 11 |
| target read back | pass | 10 parts, 63152 bytes, read-back hash sha256:654a8c3ef4de8fb32c2601ea931613caa2ec7fde1e4a5a7d97fb2dae6d3f317b, equal to the Build_Manifest's |
| Kernel events | pass | the build has no event mirror (feature `trace-ram`) |
| Kernel trace | pass | the build has no trace ring (feature `trace-ram`) |
| release timing | pass | the build has no release-timing record (feature `trace-ram`) |
| stimulus seen by the analyzer | pass | 522 rises, period 17967 to 18033 µs, pulse 114.0 to 114.5 µs |
| loopback | pass | 11 stimulus pulses answered (target counted 11); latency 70.0 to 4707.0 µs, median 71.0 µs, 0 later than 15 ms; 0 unanswered inside the run, 0 responses without a stimulus, 2 response edges before the run (flashing) and 0 after it (the reset on detach); 382 pulses before the run, 129 after; capture 9.4 s |
| dispatch timeline | pass | trace pins D1/D2/D3: Tasks executing 0.0 % of the run, the Kernel 100.0 % over 1 entries; runs per Task [0, 0, 0, 0, 0, 0, 0] |

## Commands

    probe-rs list
    sigrok-cli -d fx2lafw --scan
    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=2000000 --time 62000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/rig-flight/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/flight/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=20
sensor=9
log=5
pulse=11
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
     Finished in 6.74s
```

Build: `p1-demo` with features `nrf52840,debug-port,quiet` (sha256:654a8c3ef4de8fb32c2601ea931613caa2ec7fde1e4a5a7d97fb2dae6d3f317b, 63152 loadable bytes).
