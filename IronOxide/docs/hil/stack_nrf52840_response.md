# Stack painting: `nrf52840_response`

Binary: `sha256:0457f4dbbf6f4f9f76f46d1b006dd192d943e64a64323f7385269f07696d6eed`. Verdict: **pass**.

The rig fills every stack region with the run's pattern and resets the board in one probe command, lets the Application run, and reads the regions back: the used depth is the distance from a region's top to its lowest word without the pattern. A lower bound on the deepest use, judged against the Link_Checker's bound (R37).

| Step | Verdict | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.4 s and reset |
| target flashed | pass | in 8.6 s |
| painted and reset | pass | 4 regions filled with 0xc5a58301 and the board reset by the same command; read after 90 s: part0 0x20008000..0x20009000, part1 0x20010000..0x20011000, part2 0x20018000..0x20019000, kernel 0x20006a7c..0x20008000 |
| the Application ran | pass | 237 periodic Releases recorded when the regions were read |
| events of the run | pass | 41 mirrored in all; the held ones by kind: memmanage 1, deadline_miss 8, release_overlap 3, mit_violation 13, partition_restart 2, partition_stop 1, safe_state 1, debugger 1, endpoint_full 10, message_rejected 1 |
| regions read | pass | 4 regions |
| stack of `part0` within the Link_Checker's bound (R37) | pass | 256 bytes used of a bound of 1952 (4096 available) |
| stack of `part1` within the Link_Checker's bound (R37) | pass | 160 bytes used of a bound of 232 (4096 available) |
| stack of `part2` within the Link_Checker's bound (R37) | pass | 152 bytes used of a bound of 1816 (4096 available) |
| stack of `kernel` within the Link_Checker's bound (R37) | pass | 2268 bytes used of a bound of 3872 (5508 available) |

| Region | Range | Available | Link_Checker bound | Used | Used of the bound |
|---|---|---|---|---|---|
| part0 | 0x20008000..0x20009000 | 4096 | 1952 | 256 | 13 % |
| part1 | 0x20010000..0x20011000 | 4096 | 232 | 160 | 69 % |
| part2 | 0x20018000..0x20019000 | 4096 | 1816 | 152 | 8 % |
| kernel | 0x20006a7c..0x20008000 | 5508 | 3872 | 2268 | 59 % |

Commands:

- `probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus`
- `probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833`
- `probe-rs download --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/response/nrf52840_response/thumbv7em-none-eabihf/release/p1-demo`
- `probe-rs download --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --binary-format hex --reset /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stack/nrf52840_response/paint.hex`
