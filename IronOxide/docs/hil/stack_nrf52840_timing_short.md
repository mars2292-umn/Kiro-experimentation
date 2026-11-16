# Stack painting: `nrf52840_timing_short`

Binary: `sha256:a4d71868fb1247541050a23de69667ca56d6f9698d2acafd8608837331ad4e2e`. Verdict: **pass**.

The rig fills every stack region with the run's pattern and resets the board in one probe command, lets the Application run, and reads the regions back: the used depth is the distance from a region's top to its lowest word without the pattern. A lower bound on the deepest use, judged against the Link_Checker's bound (R37).

| Step | Verdict | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.4 s and reset |
| target flashed | pass | in 8.2 s |
| painted and reset | pass | 4 regions filled with 0xc5a529d1 and the board reset by the same command; read after 165 s: part0 0x20008000..0x20009000, part1 0x20010000..0x20011000, part2 0x20018000..0x20019000, kernel 0x2000695c..0x20008000 |
| the Application ran | pass | 37161 periodic Releases recorded when the regions were read |
| regions read | pass | 4 regions |
| stack of `part0` within the Link_Checker's bound (R37) | pass | 152 bytes used of a bound of 1960 (4096 available) |
| stack of `part1` within the Link_Checker's bound (R37) | pass | 88 bytes used of a bound of 160 (4096 available) |
| stack of `part2` within the Link_Checker's bound (R37) | pass | 112 bytes used of a bound of 1960 (4096 available) |
| stack of `kernel` within the Link_Checker's bound (R37) | pass | 2260 bytes used of a bound of 3816 (5796 available) |

| Region | Range | Available | Link_Checker bound | Used | Used of the bound |
|---|---|---|---|---|---|
| part0 | 0x20008000..0x20009000 | 4096 | 1960 | 152 | 8 % |
| part1 | 0x20010000..0x20011000 | 4096 | 160 | 88 | 55 % |
| part2 | 0x20018000..0x20019000 | 4096 | 1960 | 112 | 6 % |
| kernel | 0x2000695c..0x20008000 | 5796 | 3816 | 2260 | 59 % |

Commands:

- `probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus`
- `probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833`
- `probe-rs download --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing_short/thumbv7em-none-eabihf/release/p1-demo`
- `probe-rs download --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --binary-format hex --reset /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stack/nrf52840_timing_short/paint.hex`
