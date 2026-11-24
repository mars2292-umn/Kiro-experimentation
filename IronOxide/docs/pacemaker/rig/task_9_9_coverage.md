# Task 9.9: branch coverage of the measured items

Coverage builds (nightly-2026-09-17), each run 300 s under its records' scenario:

- set vvi-90: sha256:df8117a0b3f4cb28ad6b48f49d54a984e5aad303f3f5609284249e0bfcfb54c8; nrf52840_measure, set vvi-90, Heart_Simulator `fixed` (300 s)
- set aai-90: sha256:2f6452b619f7448097d8f5adbf13ebadeaf0640eb83182b4dedbb8781143b812; nrf52840_measure, set aai-90, Heart_Simulator `fixed` (300 s)
- set ddd-90: sha256:8b29ca174d54792a14a6bc8b9124ab9fd9cbdd4e319b9989158d47e36ce3930f; nrf52840_measure, set ddd-90, Heart_Simulator `fixed` (300 s)

A branch outcome is taken when a run of either set took it. Threshold: 50 % (`hil-rig.toml`, R34.4).

| Task entry | Branch outcomes | Taken | Percent | Of its own functions | Of the shared functions |
|---|---|---|---|---|---|
| `therapy.pace` | 194 | 143 | 73 | 9 of 10 | 134 of 184 |
| `therapy.sense_v` | 190 | 138 | 72 | 4 of 6 | 134 of 184 |
| `therapy.sense_a` | 190 | 138 | 72 | 4 of 6 | 134 of 184 |
| `therapy.control` | 314 | 203 | 64 | 69 of 130 | 134 of 184 |

Branch outcomes no run took (`line:column outcome`):

- `_<pacemaker_therapy::board_nrf::Board$u20$as$u20$pm_hw::Board>::configure` (apps/pacemaker/therapy/src/board_nrf.rs; control): 2 of 8: 166:15 false, 173:16 false
- `_<pm_hw::Driver<B>$u20$as$u20$pm_hw::Hw>::arm` (crates/pm-hw/src/lib.rs; shared): 4 of 8: 309:12 true, 318:15 false, 356:12 true, 356:12 false
- `_<pm_hw::Driver<B>$u20$as$u20$pm_hw::Hw>::disarm` (crates/pm-hw/src/lib.rs; shared): 1 of 2: 369:12 true
- `_<pm_hw::Driver<B>$u20$as$u20$pm_hw::Hw>::fired` (crates/pm-hw/src/lib.rs; shared): 3 of 12: 403:35 true, 406:27 false, 413:20 false
- `_<pm_hw::Driver<B>$u20$as$u20$pm_hw::Hw>::refresh` (crates/pm-hw/src/lib.rs; shared): 1 of 4: 384:36 false
- `_<pm_hw::Driver<B>$u20$as$u20$pm_hw::Hw>::sensed` (crates/pm-hw/src/lib.rs; shared): 2 of 8: 432:15 false, 446:12 true
- `pacemaker_therapy::Timing::init` (apps/pacemaker/therapy/src/lib.rs; control): 2 of 4: 279:12 true, 284:20 false
- `pacemaker_therapy::Timing::on_control` (apps/pacemaker/therapy/src/lib.rs; control): 1 of 4: 301:12 true
- `pacemaker_therapy::Timing::on_pace` (apps/pacemaker/therapy/src/lib.rs; pace): 1 of 10: 248:24 true
- `pacemaker_therapy::Timing::on_sense` (apps/pacemaker/therapy/src/lib.rs; sense_v, sense_a): 2 of 6: 263:25 true, 267:16 false
- `pacemaker_therapy::sets::from_text` (apps/pacemaker/therapy/src/sets.rs; not counted: evaluated by the compiler only): 24 of 24: 52:18 true, 52:18 false, 52:33 true, 52:33 false, 54:12 true, 54:12 false, 54:25 true, 54:25 false, 58:16 true, 58:16 false, 59:20 true, 59:20 false, 62:23 true, 62:23 false, 65:19 true, 65:19 false, 68:12 true, 68:12 false, 68:25 true, 68:25 false, 69:16 true, 69:16 false, 69:28 true, 69:28 false
- `pm_core::state::State::params` (crates/pm-core/src/state.rs; shared): 1 of 2: 202:35 false
- `pm_core::step::atrial_event` (crates/pm-core/src/step.rs; shared): 4 of 20: 244:21 false, 248:12 false, 252:20 true, 262:25 false
- `pm_core::step::blanked` (crates/pm-core/src/step.rs; shared): 1 of 2: 442:32 true
- `pm_core::step::coupled` (crates/pm-core/src/step.rs; shared): 4 of 4: 163:5 true, 163:5 false, 163:30 true, 163:30 false
- `pm_core::step::is_pvc` (crates/pm-core/src/step.rs; shared): 1 of 6: 171:5 false
- `pm_core::step::paced_av` (crates/pm-core/src/step.rs; shared): 13 of 14: 118:8 false, 118:25 true, 118:25 false, 118:45 true, 118:45 false, 118:64 true, 118:64 false, 123:21 true, 123:21 false, 124:8 true, 124:8 false, 126:15 true, 126:15 false
- `pm_core::step::sense` (crates/pm-core/src/step.rs; shared): 3 of 10: 470:8 true, 482:37 true, 482:37 false
- `pm_core::step::step` (crates/pm-core/src/step.rs; shared): 5 of 16: 558:23 false, 558:40 true, 558:61 true, 558:61 false, 576:67 false
- `pm_core::step::tick` (crates/pm-core/src/step.rs; shared): 1 of 12: 413:12 false
- `pm_core::step::ventricular_event` (crates/pm-core/src/step.rs; shared): 5 of 16: 197:21 false, 199:8 false, 202:29 false, 202:48 true, 205:24 true
- `pm_core::time::Micros::elapsed_since` (crates/pm-core/src/time.rs; shared): 1 of 2: 100:12 false
- `pm_core::validate::is_amplitude` (crates/pm-core/src/validate.rs; control): 2 of 4: 237:5 true, 237:20 true
- `pm_core::validate::is_atr_duration` (crates/pm-core/src/validate.rs; control): 2 of 4: 261:5 true, 261:17 false
- `pm_core::validate::is_lrl` (crates/pm-core/src/validate.rs; control): 2 of 4: 229:5 true, 229:32 false
- `pm_core::validate::is_rate_smoothing` (crates/pm-core/src/validate.rs; control): 3 of 4: 269:5 false, 269:21 true, 269:21 false
- `pm_core::validate::is_sensitivity` (crates/pm-core/src/validate.rs; control): 2 of 6: 253:5 true, 253:25 true
- `pm_core::validate::is_width` (crates/pm-core/src/validate.rs; control): 1 of 2: 245:5 true
- `pm_core::validate::limits_hold` (crates/pm-core/src/validate.rs; control): 5 of 14: 330:9 false, 331:16 false, 332:16 false, 333:17 false, 338:5 false
- `pm_core::validate::stepped` (crates/pm-core/src/validate.rs; control): 1 of 4: 221:5 false
- `pm_core::validate::validate` (crates/pm-core/src/validate.rs; control): 38 of 72: 356:8 true, 360:18 false, 361:12 false, 362:12 false, 363:12 false, 364:12 false, 365:12 false, 366:13 false, 366:47 true, 366:47 false, 367:12 false, 368:12 false, 369:12 false, 370:12 false, 371:12 false, 372:12 false, 373:12 false, 374:12 false, 375:12 false, 376:13 false, 376:46 true, 376:46 false, 377:13 false, 377:34 true, 377:34 false, 378:12 false, 379:12 false, 380:12 false, 381:12 false, 382:12 false, 383:12 false, 384:12 false, 386:8 true, 392:8 true, 399:8 true, 399:34 true, 399:55 true, 399:105 true

Verdict: **pass**
