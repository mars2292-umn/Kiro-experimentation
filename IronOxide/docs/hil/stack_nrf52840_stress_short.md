# Stack painting: `nrf52840_stress_short`

Binary: `sha256:cc4cb4b7b31b7971fe5c74947fe017c8703cf9ffd91300edad0aca7463543547`. Verdict: **pass**.

The rig fills every stack region with the run's pattern and resets the board in one probe command, lets the Application run, and reads the regions back: the used depth is the distance from a region's top to its lowest word without the pattern. A lower bound on the deepest use, judged against the Link_Checker's bound (R37).

| Step | Verdict | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.5 s and reset |
| target flashed | pass | in 8.6 s |
| painted and reset | pass | 4 regions filled with 0xc5a59906 and the board reset by the same command; read after 165 s: part0 0x20008000..0x20009000, part1 0x20010000..0x20011000, part2 0x20018000..0x20019000, kernel 0x20006c74..0x20008000 |
| the Application ran | pass | 37175 periodic Releases recorded when the regions were read |
| events of the run | pass | 49 mirrored in all; the held ones by kind: deadline_miss 15, release_overlap 4, mit_violation 27, partition_restart 2, debugger 1 |
| regions read | pass | 4 regions |
| stack of `part0` within the Link_Checker's bound (R37) | pass | 152 bytes used of a bound of 1896 (4096 available) |
| stack of `part1` within the Link_Checker's bound (R37) | pass | 144 bytes used of a bound of 280 (4096 available) |
| stack of `part2` within the Link_Checker's bound (R37) | pass | 336 bytes used of a bound of 2024 (4096 available) |
| stack of `kernel` within the Link_Checker's bound (R37) | pass | 2260 bytes used of a bound of 3888 (5004 available) |

| Region | Range | Available | Link_Checker bound | Used | Used of the bound |
|---|---|---|---|---|---|
| part0 | 0x20008000..0x20009000 | 4096 | 1896 | 152 | 8 % |
| part1 | 0x20010000..0x20011000 | 4096 | 280 | 144 | 51 % |
| part2 | 0x20018000..0x20019000 | 4096 | 2024 | 336 | 17 % |
| kernel | 0x20006c74..0x20008000 | 5004 | 3888 | 2260 | 58 % |

Commands:

- `probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stimulus-storm/thumbv7em-none-eabihf/release/hil-stimulus`
- `probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833`
- `probe-rs download --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress_short/thumbv7em-none-eabihf/release/p1-demo`
- `probe-rs download --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --binary-format hex --reset /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stack/nrf52840_stress_short/paint.hex`
