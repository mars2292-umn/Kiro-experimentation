# Stack painting: `nrf52840_isolation`

Binary: `sha256:2bf9540e23cee2ee0a5a67c5930008d3047d5cc97537ca8480819806c9d72ea6`. Verdict: **pass**.

The rig fills every stack region with the run's pattern and resets the board in one probe command, lets the Application run, and reads the regions back: the used depth is the distance from a region's top to its lowest word without the pattern. A lower bound on the deepest use, judged against the Link_Checker's bound (R37).

| Step | Verdict | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.2 s and reset |
| target flashed | pass | in 8.5 s |
| painted and reset | pass | 4 regions filled with 0xc5a58371 and the board reset by the same command; read after 60 s: part0 0x20008000..0x20009000, part1 0x20010000..0x20011000, part2 0x20018000..0x20019000, kernel 0x20006c9c..0x20008000 |
| the Application ran | pass | 792 periodic Releases recorded when the regions were read |
| events of the run | pass | 177 mirrored in all; the held ones by kind: memmanage 66, busfault 6, deadline_miss 6, release_overlap 1, mit_violation 13, operating_end 1, dma_rejected 5, safe_state 1, end_job 77, debugger 1 |
| regions read | pass | 4 regions |
| stack of `part0` within the Link_Checker's bound (R37) | pass | 312 bytes used of a bound of 3124 (4096 available) |
| stack of `part1` within the Link_Checker's bound (R37) | pass | 192 bytes used of a bound of 2184 (4096 available) |
| stack of `part2` within the Link_Checker's bound (R37) | pass | 256 bytes used of a bound of 3188 (4096 available) |
| stack of `kernel` within the Link_Checker's bound (R37) | pass | 2260 bytes used of a bound of 3872 (4964 available) |

| Region | Range | Available | Link_Checker bound | Used | Used of the bound |
|---|---|---|---|---|---|
| part0 | 0x20008000..0x20009000 | 4096 | 3124 | 312 | 10 % |
| part1 | 0x20010000..0x20011000 | 4096 | 2184 | 192 | 9 % |
| part2 | 0x20018000..0x20019000 | 4096 | 3188 | 256 | 8 % |
| kernel | 0x20006c9c..0x20008000 | 4964 | 3872 | 2260 | 58 % |

Commands:

- `probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus`
- `probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833`
- `probe-rs download --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/isolation/nrf52840_isolation/thumbv7em-none-eabihf/release/p1-demo`
- `probe-rs download --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --binary-format hex --reset /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stack/nrf52840_isolation/paint.hex`
