# Task 9.9: branch coverage of the measured items

Coverage builds (nightly-2026-09-17), each run 300 s under its records' scenario:

- set vvi-90: sha256:6d63cd1dca50079585c76afc5ea07dc487dd104b5fc3e5a986bc876e2901b706; nrf52840_measure, set vvi-90, Heart_Simulator `fixed` (300 s)
- set aai-90: sha256:a93abaec73b9cec5b9470656b235e4c842fb95b7f62cb3996c3e6e2c71bb7303; nrf52840_measure, set aai-90, Heart_Simulator `fixed` (300 s)

A branch outcome is taken when a run of either set took it. Threshold: 50 % (`hil-rig.toml`, R34.4).

| Task entry | Branch outcomes | Taken | Percent | Of its own functions | Of the shared functions |
|---|---|---|---|---|---|
| `therapy.pace` | 92 | 59 | 64 | 8 of 10 | 51 of 82 |
| `therapy.sense_v` | 88 | 54 | 61 | 3 of 6 | 51 of 82 |
| `therapy.sense_a` | 88 | 54 | 61 | 3 of 6 | 51 of 82 |
| `therapy.control` | 194 | 108 | 55 | 57 of 112 | 51 of 82 |

Branch outcomes no run took (`line:column outcome`):

- `_<pacemaker_therapy::board_nrf::Board$u20$as$u20$pm_hw::Board>::configure` (apps/pacemaker/therapy/src/board_nrf.rs; control): 2 of 8: 166:15 false, 173:16 false
- `_<pm_hw::Driver<B>$u20$as$u20$pm_hw::Hw>::arm` (crates/pm-hw/src/lib.rs; shared): 8 of 12: 283:12 true, 289:15 false, 290:16 true, 307:16 false, 315:12 true, 315:12 false, 328:12 true, 328:12 false
- `_<pm_hw::Driver<B>$u20$as$u20$pm_hw::Hw>::disarm` (crates/pm-hw/src/lib.rs; shared): 4 of 4: 341:12 true, 341:12 false, 344:12 true, 344:12 false
- `_<pm_hw::Driver<B>$u20$as$u20$pm_hw::Hw>::fired` (crates/pm-hw/src/lib.rs; shared): 3 of 12: 372:35 true, 375:27 false, 382:20 false
- `_<pm_hw::Driver<B>$u20$as$u20$pm_hw::Hw>::refresh` (crates/pm-hw/src/lib.rs; shared): 1 of 4: 355:36 false
- `_<pm_hw::Driver<B>$u20$as$u20$pm_hw::Hw>::sensed` (crates/pm-hw/src/lib.rs; shared): 2 of 8: 399:15 false, 413:12 true
- `pacemaker_therapy::Timing::init` (apps/pacemaker/therapy/src/lib.rs; control): 2 of 4: 273:12 true, 278:20 false
- `pacemaker_therapy::Timing::on_control` (apps/pacemaker/therapy/src/lib.rs; control): 1 of 4: 295:12 true
- `pacemaker_therapy::Timing::on_pace` (apps/pacemaker/therapy/src/lib.rs; pace): 2 of 10: 241:49 true, 242:24 true
- `pacemaker_therapy::Timing::on_sense` (apps/pacemaker/therapy/src/lib.rs; sense_v, sense_a): 3 of 6: 257:12 true, 257:25 true, 261:16 false
- `pacemaker_therapy::event` (apps/pacemaker/therapy/src/lib.rs; shared): 1 of 6: 209:24 true
- `pacemaker_therapy::sets::from_text` (apps/pacemaker/therapy/src/sets.rs; not counted: evaluated by the compiler only): 24 of 24: 52:18 true, 52:18 false, 52:33 true, 52:33 false, 54:12 true, 54:12 false, 54:25 true, 54:25 false, 58:16 true, 58:16 false, 59:20 true, 59:20 false, 62:23 true, 62:23 false, 65:19 true, 65:19 false, 68:12 true, 68:12 false, 68:25 true, 68:25 false, 69:16 true, 69:16 false, 69:28 true, 69:28 false
- `pm_core::state::State::params` (crates/pm-core/src/state.rs; shared): 1 of 2: 201:35 false
- `pm_core::step::account` (crates/pm-core/src/step.rs; shared): 1 of 4: 56:25 true
- `pm_core::step::apply_pace` (crates/pm-core/src/step.rs; shared): 1 of 2: 47:21 false
- `pm_core::step::arm_from` (crates/pm-core/src/step.rs; shared): 1 of 2: 94:8 false
- `pm_core::step::step` (crates/pm-core/src/step.rs; shared): 7 of 16: 134:28 true, 134:47 false, 138:16 false, 138:35 true, 138:35 false, 153:16 false, 162:16 false
- `pm_core::step::tick` (crates/pm-core/src/step.rs; shared): 1 of 6: 111:15 true
- `pm_core::validate::delivered_in_e1` (crates/pm-core/src/validate.rs; control): 1 of 2: 170:5 false
- `pm_core::validate::is_amplitude` (crates/pm-core/src/validate.rs; control): 2 of 4: 116:5 true, 116:20 true
- `pm_core::validate::is_atr_duration` (crates/pm-core/src/validate.rs; control): 2 of 4: 137:5 true, 137:17 false
- `pm_core::validate::is_lrl` (crates/pm-core/src/validate.rs; control): 2 of 4: 109:5 true, 109:32 false
- `pm_core::validate::is_rate_smoothing` (crates/pm-core/src/validate.rs; control): 3 of 4: 144:5 false, 144:21 true, 144:21 false
- `pm_core::validate::is_sensitivity` (crates/pm-core/src/validate.rs; control): 2 of 6: 130:5 true, 130:25 true
- `pm_core::validate::is_width` (crates/pm-core/src/validate.rs; control): 1 of 2: 123:5 true
- `pm_core::validate::stepped` (crates/pm-core/src/validate.rs; control): 1 of 4: 102:5 false
- `pm_core::validate::validate` (crates/pm-core/src/validate.rs; control): 36 of 66: 183:8 true, 187:18 false, 188:12 false, 189:12 false, 190:12 false, 191:12 false, 192:12 false, 193:13 false, 193:47 true, 193:47 false, 194:12 false, 195:12 false, 196:12 false, 197:12 false, 198:12 false, 199:12 false, 200:12 false, 201:12 false, 202:12 false, 203:13 false, 203:46 true, 203:46 false, 204:13 false, 204:34 true, 204:34 false, 205:12 false, 206:12 false, 207:12 false, 208:12 false, 209:12 false, 210:12 false, 211:12 false, 213:8 true, 219:8 true, 219:34 true, 219:55 true

Verdict: **pass**
