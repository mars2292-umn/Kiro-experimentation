# Stack painting: `nrf52840_stress_short`

Binary: `sha256:2b371e899dfccda96fe50601e7257cd1bdce41467ef36669bae2cf68cd2e053b`. Verdict: **pass**.

The rig fills every stack region with the run's pattern and resets the board in one probe command, lets the Application run, and reads the regions back: the used depth is the distance from a region's top to its lowest word without the pattern. A lower bound on the deepest use, judged against the Link_Checker's bound (R37).

| Step | Verdict | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.5 s and reset |
| target flashed | pass | in 8.2 s |
| painted and reset | pass | 4 regions filled with 0xc5a56e00 and the board reset by the same command; read after 165 s: part0 0x20008000..0x20009000, part1 0x20010000..0x20011000, part2 0x20018000..0x20019000, kernel 0x20006a5c..0x20008000 |
| the Application ran | pass | 37178 periodic Releases recorded when the regions were read |
| events of the run | pass | 47 mirrored in all; the held ones by kind: deadline_miss 17, release_overlap 3, mit_violation 25, partition_restart 1, debugger 1 |
| regions read | pass | 4 regions |
| stack of `part0` within the Link_Checker's bound (R37) | pass | 152 bytes used of a bound of 1896 (4096 available) |
| stack of `part1` within the Link_Checker's bound (R37) | pass | 176 bytes used of a bound of 280 (4096 available) |
| stack of `part2` within the Link_Checker's bound (R37) | pass | 336 bytes used of a bound of 2024 (4096 available) |
| stack of `kernel` within the Link_Checker's bound (R37) | pass | 2268 bytes used of a bound of 3896 (5540 available) |

| Region | Range | Available | Link_Checker bound | Used | Used of the bound |
|---|---|---|---|---|---|
| part0 | 0x20008000..0x20009000 | 4096 | 1896 | 152 | 8 % |
| part1 | 0x20010000..0x20011000 | 4096 | 280 | 176 | 63 % |
| part2 | 0x20018000..0x20019000 | 4096 | 2024 | 336 | 17 % |
| kernel | 0x20006a5c..0x20008000 | 5540 | 3896 | 2268 | 58 % |

Commands:

- `probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stimulus-storm/thumbv7em-none-eabihf/release/hil-stimulus`
- `probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833`
- `probe-rs download --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress_short/thumbv7em-none-eabihf/release/p1-demo`
- `probe-rs download --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --binary-format hex --reset /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stack/nrf52840_stress_short/paint.hex`
