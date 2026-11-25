# Task 9.9: branch coverage of the measured items

Coverage builds (nightly-2026-09-17), each run 300 s under its records' scenario:

- set vvi-90: sha256:3538ee39a9385e091d6bfe9bed9f06827e76056dc0f0e2f69e3ee373e660df55; nrf52840_measure, set vvi-90, Heart_Simulator `fixed` (300 s)
- set aai-90: sha256:be3f8046048f18fe11e791ccf291ecc711e9889995dd8228763e15dde46d0453; nrf52840_measure, set aai-90, Heart_Simulator `fixed` (300 s)
- set ddd-90: sha256:ff222877b1d22e07cb471dc29a057df69699feccf21d0e3e48cb3dd05b353a0e; nrf52840_measure, set ddd-90, Heart_Simulator `fixed` (300 s)
- set dddr-e4: sha256:79f8bd807aa9189f33235eb6cd5245f3152a36bdef024f27fe68889c5d26b802; nrf52840_measure, set dddr-e4, Heart_Simulator `fixed` (300 s)

A branch outcome is taken when a run of either set took it. Threshold: 50 % (`hil-rig.toml`, R34.4).

| Task entry | Branch outcomes | Taken | Percent | Of its own functions | Of the shared functions |
|---|---|---|---|---|---|
| `therapy.pace` | 622 | 337 | 54 | 9 of 12 | 328 of 610 |
| `therapy.sense_v` | 618 | 332 | 53 | 4 of 8 | 328 of 610 |
| `therapy.sense_a` | 618 | 332 | 53 | 4 of 8 | 328 of 610 |
| `therapy.control` | 800 | 434 | 54 | 106 of 190 | 328 of 610 |

Branch outcomes no run took (`line:column outcome`):

- `_<pacemaker_therapy::board_nrf::Board$u20$as$u20$pm_hw::Board>::configure` (apps/pacemaker/therapy/src/board_nrf.rs; control): 2 of 8: 181:15 false, 188:16 false
- `_<pm_hw::Driver<B>$u20$as$u20$pm_hw::Hw>::arm` (crates/pm-hw/src/lib.rs; shared): 5 of 8: 375:12 true, 386:15 false, 403:16 false, 426:12 true, 426:12 false
- `_<pm_hw::Driver<B>$u20$as$u20$pm_hw::Hw>::disarm` (crates/pm-hw/src/lib.rs; shared): 1 of 2: 439:12 true
- `_<pm_hw::Driver<B>$u20$as$u20$pm_hw::Hw>::fired` (crates/pm-hw/src/lib.rs; shared): 1 of 12: 473:35 true
- `_<pm_hw::Driver<B>$u20$as$u20$pm_hw::Hw>::refresh` (crates/pm-hw/src/lib.rs; shared): 1 of 4: 454:36 false
- `_<pm_hw::Driver<B>$u20$as$u20$pm_hw::Hw>::sensed` (crates/pm-hw/src/lib.rs; shared): 2 of 8: 502:15 false, 516:12 true
- `pacemaker_comms::link` (apps/pacemaker/comms/src/lib.rs; shared): 6 of 24: 170:8 true, 205:12 true, 206:12 true, 206:12 false, 220:16 false, 223:8 false
- `pacemaker_comms::proto::answer` (apps/pacemaker/comms/src/proto.rs; shared): 26 of 40: 248:20 false, 266:8 true, 274:8 true, 274:40 true, 275:12 true, 275:12 false, 275:27 true, 275:27 false, 280:12 true, 280:12 false, 280:40 true, 280:40 false, 290:20 true, 290:20 false, 303:8 true, 304:12 true, 304:12 false, 313:8 true, 313:41 true, 322:8 true, 322:39 false, 322:71 true, 322:71 false, 323:12 true, 327:12 true, 330:12 true
- `pacemaker_comms::proto::idle_or_part` (apps/pacemaker/comms/src/proto.rs; shared): 10 of 20: 223:8 true, 223:28 true, 223:28 false, 223:58 true, 223:58 false, 225:26 true, 225:26 false, 226:16 true, 226:16 false, 232:87 true
- `pacemaker_comms::proto::peer_changed` (apps/pacemaker/comms/src/proto.rs; shared): 1 of 2: 146:8 true
- `pacemaker_comms::proto::status` (apps/pacemaker/comms/src/proto.rs; shared): 1 of 4: 196:8 false
- `pacemaker_comms::proto::take_resend` (apps/pacemaker/comms/src/proto.rs; shared): 1 of 2: 156:8 true
- `pacemaker_therapy::Timing::init` (apps/pacemaker/therapy/src/lib.rs; control): 3 of 6: 419:12 true, 424:26 false, 425:20 false
- `pacemaker_therapy::Timing::on_control` (apps/pacemaker/therapy/src/lib.rs; control): 28 of 48: 453:12 true, 465:12 true, 477:16 true, 480:23 false, 483:23 true, 483:23 false, 485:23 true, 485:23 false, 487:23 true, 487:23 false, 489:23 true, 489:23 false, 491:23 true, 491:23 false, 496:43 false, 499:12 false, 501:16 true, 508:20 true, 508:20 false, 510:27 true, 510:27 false, 521:20 true, 521:20 false, 527:24 true, 529:24 true, 529:24 false, 529:70 true, 529:70 false
- `pacemaker_therapy::Timing::on_pace` (apps/pacemaker/therapy/src/lib.rs; pace): 3 of 12: 381:12 true, 387:49 true, 388:24 true
- `pacemaker_therapy::Timing::on_sense` (apps/pacemaker/therapy/src/lib.rs; sense_v, sense_a): 4 of 8: 403:12 true, 403:25 true, 403:38 true, 407:16 false
- `pacemaker_therapy::control` (apps/pacemaker/therapy/src/lib.rs; control): 1 of 10: 624:42 true
- `pacemaker_therapy::event` (apps/pacemaker/therapy/src/lib.rs; shared): 1 of 6: 343:24 true
- `pacemaker_therapy::sets::from_text` (apps/pacemaker/therapy/src/sets.rs; not counted: evaluated by the compiler only): 24 of 24: 52:18 true, 52:18 false, 52:33 true, 52:33 false, 54:12 true, 54:12 false, 54:25 true, 54:25 false, 58:16 true, 58:16 false, 59:20 true, 59:20 false, 62:23 true, 62:23 false, 65:19 true, 65:19 false, 68:12 true, 68:12 false, 68:25 true, 68:25 false, 69:16 true, 69:16 false, 69:28 true, 69:28 false
- `pm_core::frame::Assembly::accept` (crates/pm-core/src/frame.rs; shared): 12 of 12: 256:12 true, 256:12 false, 256:26 true, 256:26 false, 260:12 true, 260:12 false, 267:15 true, 267:15 false, 268:16 true, 268:16 false, 279:12 true, 279:12 false
- `pm_core::frame::decode` (crates/pm-core/src/frame.rs; shared): 7 of 20: 170:37 true, 174:8 true, 174:29 true, 174:46 true, 174:72 true, 174:86 true, 184:19 true
- `pm_core::frame::encode` (crates/pm-core/src/frame.rs; shared): 1 of 6: 142:18 true
- `pm_core::frame::set_part` (crates/pm-core/src/frame.rs; shared): 6 of 6: 213:8 true, 213:8 false, 221:11 true, 221:11 false, 222:12 true, 222:12 false
- `pm_core::frame::words_in_part` (crates/pm-core/src/frame.rs; shared): 4 of 4: 197:8 true, 197:8 false, 199:15 true, 199:15 false
- `pm_core::mode::Mode::timing_chamber` (crates/pm-core/src/mode.rs; shared): 4 of 4: 250:12 true, 250:12 false, 250:38 true, 250:38 false
- `pm_core::mode::Mode::uses_rate_smoothing` (crates/pm-core/src/mode.rs; shared): 1 of 2: 235:9 false
- `pm_core::rate::fallback_interval` (crates/pm-core/src/rate.rs; shared): 8 of 8: 152:8 true, 152:8 false, 152:24 true, 152:24 false, 152:52 true, 152:52 false, 158:8 true, 158:8 false
- `pm_core::rate::interval_of` (crates/pm-core/src/rate.rs; shared): 2 of 4: 47:8 true, 51:8 true
- `pm_core::rate::rate_of` (crates/pm-core/src/rate.rs; shared): 2 of 4: 31:8 true, 35:8 true
- `pm_core::rate::sensor_next` (crates/pm-core/src/rate.rs; shared): 8 of 20: 111:8 true, 116:15 true, 124:23 true, 125:23 true, 127:12 true, 133:23 true, 134:23 true, 135:12 false
- `pm_core::rate::smoothed` (crates/pm-core/src/rate.rs; shared): 14 of 14: 172:20 true, 172:20 false, 173:22 true, 173:22 false, 176:8 true, 176:8 false, 176:19 true, 176:19 false, 178:15 true, 178:15 false, 178:28 true, 178:28 false, 179:12 true, 179:12 false
- `pm_core::rate::target` (crates/pm-core/src/rate.rs; shared): 3 of 8: 88:8 true, 91:21 true, 94:8 true
- `pm_core::state::State::own` (crates/pm-core/src/state.rs; shared): 1 of 2: 286:35 false
- `pm_core::state::State::params` (crates/pm-core/src/state.rs; shared): 1 of 2: 267:12 true
- `pm_core::step::atr_atrial` (crates/pm-core/src/step.rs; shared): 11 of 16: 435:20 true, 438:8 true, 439:12 true, 439:12 false, 440:16 true, 440:16 false, 443:19 true, 443:19 false, 450:27 true, 455:31 true, 455:31 false
- `pm_core::step::atr_ventricular` (crates/pm-core/src/step.rs; shared): 7 of 10: 402:8 true, 410:16 true, 410:16 false, 413:43 true, 413:43 false, 417:16 true, 417:16 false
- `pm_core::step::atrial_event` (crates/pm-core/src/step.rs; shared): 7 of 30: 501:21 false, 505:12 false, 509:20 true, 513:16 true, 521:12 false, 522:46 true, 529:53 false
- `pm_core::step::blanked` (crates/pm-core/src/step.rs; shared): 1 of 2: 729:32 true
- `pm_core::step::coupled` (crates/pm-core/src/step.rs; shared): 4 of 4: 315:5 true, 315:5 false, 315:30 true, 315:30 false
- `pm_core::step::depleted` (crates/pm-core/src/step.rs; shared): 6 of 6: 739:8 true, 739:8 false, 741:15 true, 741:15 false, 741:29 true, 741:29 false
- `pm_core::step::escape` (crates/pm-core/src/step.rs; shared): 8 of 26: 238:46 false, 240:27 false, 245:12 true, 245:12 false, 249:37 true, 249:68 true, 253:8 true, 256:8 true
- `pm_core::step::in_force` (crates/pm-core/src/step.rs; shared): 3 of 10: 774:8 true, 788:8 true, 791:8 true
- `pm_core::step::is_pvc` (crates/pm-core/src/step.rs; shared): 1 of 6: 323:5 false
- `pm_core::step::paced_av` (crates/pm-core/src/step.rs; shared): 13 of 14: 270:8 false, 270:25 true, 270:25 false, 270:45 true, 270:45 false, 270:64 true, 270:64 false, 275:21 true, 275:21 false, 276:8 true, 276:8 false, 278:15 true, 278:15 false
- `pm_core::step::rearmed` (crates/pm-core/src/step.rs; shared): 2 of 2: 604:8 true, 604:8 false
- `pm_core::step::resolve` (crates/pm-core/src/step.rs; shared): 13 of 18: 828:8 true, 840:21 true, 840:44 true, 840:44 false, 842:31 true, 843:31 true, 843:31 false, 846:8 false, 849:8 true, 849:21 true, 849:21 false, 849:39 true, 849:39 false
- `pm_core::step::sense` (crates/pm-core/src/step.rs; shared): 3 of 10: 972:8 true, 984:37 true, 984:37 false
- `pm_core::step::serviced` (crates/pm-core/src/step.rs; shared): 2 of 14: 931:35 true, 935:17 true
- `pm_core::step::step` (crates/pm-core/src/step.rs; shared): 30 of 50: 1069:47 false, 1070:16 true, 1070:37 true, 1070:37 false, 1081:23 false, 1081:40 true, 1081:61 true, 1081:61 false, 1099:67 false, 1112:16 true, 1118:16 true, 1121:16 true, 1121:41 true, 1121:41 false, 1121:66 true, 1121:66 false, 1121:92 true, 1121:92 false, 1121:126 true, 1121:126 false, 1130:16 true, 1130:25 true, 1130:25 false, 1155:16 true, 1190:62 true, 1190:62 false, 1190:76 true, 1190:76 false, 1202:24 true, 1202:24 false
- `pm_core::step::switched` (crates/pm-core/src/step.rs; shared): 6 of 6: 898:30 true, 898:30 false, 898:39 true, 898:39 false, 898:70 true, 898:70 false
- `pm_core::step::tick` (crates/pm-core/src/step.rs; shared): 1 of 12: 699:12 false
- `pm_core::step::ventricular_event` (crates/pm-core/src/step.rs; shared): 6 of 22: 359:8 false, 366:49 false, 368:8 false, 371:29 false, 371:48 true, 374:24 true
- `pm_core::step::waiting` (crates/pm-core/src/step.rs; shared): 4 of 8: 867:5 true, 868:12 true, 869:12 true, 870:12 true
- `pm_core::time::Micros::elapsed_since` (crates/pm-core/src/time.rs; shared): 2 of 4: 114:12 true, 118:16 true
- `pm_core::time::Micros::minus` (crates/pm-core/src/time.rs; shared): 1 of 2: 64:19 false
- `pm_core::time::Micros::plus` (crates/pm-core/src/time.rs; shared): 1 of 2: 56:19 true
- `pm_core::validate::is_amplitude` (crates/pm-core/src/validate.rs; control): 2 of 4: 236:5 true, 236:20 true
- `pm_core::validate::is_atr_duration` (crates/pm-core/src/validate.rs; control): 1 of 4: 260:17 false
- `pm_core::validate::is_lrl` (crates/pm-core/src/validate.rs; control): 1 of 4: 228:32 false
- `pm_core::validate::is_rate_smoothing` (crates/pm-core/src/validate.rs; control): 1 of 4: 268:21 false
- `pm_core::validate::is_sensitivity` (crates/pm-core/src/validate.rs; control): 2 of 6: 252:5 true, 252:25 true
- `pm_core::validate::is_width` (crates/pm-core/src/validate.rs; control): 1 of 2: 244:5 true
- `pm_core::validate::limits_hold` (crates/pm-core/src/validate.rs; control): 8 of 24: 333:9 false, 334:16 false, 335:16 false, 336:17 false, 342:22 true, 343:5 false, 343:17 false, 343:27 false
- `pm_core::validate::stepped` (crates/pm-core/src/validate.rs; control): 1 of 4: 220:5 false
- `pm_core::validate::validate` (crates/pm-core/src/validate.rs; control): 33 of 66: 361:8 true, 365:18 false, 366:12 false, 367:12 false, 368:12 false, 369:12 false, 370:12 false, 371:13 false, 371:47 true, 371:47 false, 372:12 false, 373:12 false, 374:12 false, 375:12 false, 376:12 false, 377:12 false, 378:12 false, 379:12 false, 380:12 false, 381:13 false, 381:46 true, 381:46 false, 382:34 false, 383:12 false, 384:12 false, 385:12 false, 386:12 false, 387:12 false, 388:12 false, 389:12 false, 390:12 false, 392:8 true, 398:8 true
- `pm_core::wire::admitted` (crates/pm-core/src/wire.rs; shared): 16 of 28: 140:8 true, 145:8 true, 145:30 true, 149:20 true, 150:12 true, 150:12 false, 155:12 true, 159:15 true, 160:12 true, 160:12 false, 164:15 false, 164:35 true, 164:35 false, 164:63 true, 164:63 false, 173:12 true
- `pm_core::wire::message_with_set` (crates/pm-core/src/wire.rs; shared): 2 of 2: 81:11 true, 81:11 false
- `pm_hw::Driver<B>::disarm_and_check` (crates/pm-hw/src/lib.rs; shared): 1 of 2: 310:12 false
- `pm_hw::Driver<B>::extend` (crates/pm-hw/src/lib.rs; shared): 1 of 2: 294:12 true

Verdict: **pass**
