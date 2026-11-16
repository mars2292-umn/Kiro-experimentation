# Stack painting: `nrf52840_measure`

Binary: `sha256:d44020a9efabad6848227822ea8eb9e3e5cf438ccbaef7d39619e3c07959ebf1`. Verdict: **pass**.

The rig fills every stack region with the run's pattern and resets the board in one probe command, lets the Application run, and reads the regions back: the used depth is the distance from a region's top to its lowest word without the pattern. A lower bound on the deepest use, judged against the Link_Checker's bound (R37).

| Step | Verdict | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.2 s and reset |
| target flashed | pass | in 8.2 s |
| painted and reset | pass | 4 regions filled with 0xc5a52a8c and the board reset by the same command; read after 165 s: part0 0x20008000..0x20009000, part1 0x20010000..0x20011000, part2 0x20018000..0x20019000, kernel 0x20006a44..0x20008000 |
| the Application ran | pass | 37171 periodic Releases recorded when the regions were read |
| regions read | pass | 4 regions |
| stack of `part0` within the Link_Checker's bound (R37) | pass | 280 bytes used of a bound of 2088 (4096 available) |
| stack of `part1` within the Link_Checker's bound (R37) | pass | 184 bytes used of a bound of 256 (4096 available) |
| stack of `part2` within the Link_Checker's bound (R37) | pass | 208 bytes used of a bound of 2056 (4096 available) |
| stack of `kernel` within the Link_Checker's bound (R37) | pass | 2268 bytes used of a bound of 3928 (5564 available) |

| Region | Range | Available | Link_Checker bound | Used | Used of the bound |
|---|---|---|---|---|---|
| part0 | 0x20008000..0x20009000 | 4096 | 2088 | 280 | 13 % |
| part1 | 0x20010000..0x20011000 | 4096 | 256 | 184 | 72 % |
| part2 | 0x20018000..0x20019000 | 4096 | 2056 | 208 | 10 % |
| kernel | 0x20006a44..0x20008000 | 5564 | 3928 | 2268 | 58 % |

Commands:

- `probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus`
- `probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833`
- `probe-rs download --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/measure/nrf52840_measure/thumbv7em-none-eabihf/release/p1-demo`
- `probe-rs download --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --binary-format hex --reset /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stack/nrf52840_measure/paint.hex`
