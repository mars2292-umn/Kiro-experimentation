# Implementation Plan: Rocket Launch System GUI Simulator

## Overview

Build a single-file PyQt6 desktop application (`rls_gui.py`) that simulates a two-unit rocket launch system. Implementation follows a bottom-up approach: data model → reusable widgets → canvas → main window with state machine → wiring and polish. Property-based tests use Hypothesis to verify the 9 correctness properties from the design.

## Tasks

- [ ] 1. Set up project structure, State class, and helper functions
  - [~] 1.1 Create `rls_gui.py` with imports, `State` class (IDLE, CIRCUIT_TESTED, ENABLED, READY, LAUNCHED), `COMM_DELAYS` dict, and `VALID_TRANSITIONS` map
    - Import PyQt6 modules: QtWidgets, QtCore, QtGui, QGraphicsView, QGraphicsScene, QTimer, etc.
    - Define `State` class with five string constants
    - Define `COMM_DELAYS` configuration dict with test_circuit=600, ready_ack=400, launch_ack=500
    - Define `VALID_TRANSITIONS` dict mapping each state to its valid target states
    - _Requirements: 5.1, 11.1, 11.2, 11.3, 11.4, 12.1, 12.2, 12.3_

  - [~] 1.2 Implement `make_panel(title, color)` and `make_button(label, color)` helper functions
    - `make_panel` returns a styled `QFrame` with `QVBoxLayout`, title label, and separator line
    - `make_button` returns a styled `QPushButton` with hover and disabled state stylesheets
    - _Requirements: 16.2, 16.3_

- [ ] 2. Implement Light indicator widget
  - [~] 2.1 Implement `Light(QWidget)` class with `__init__`, `set_on`, and `is_on` property
    - Constructor takes `color_on: QColor` and `label: str`
    - Off state renders dim dark circle `QColor(40, 40, 40)` with `#555` border
    - On state renders bright circle in `color_on` with CSS glow effect
    - Fixed size 54×70 pixels (44×44 bulb + label beneath)
    - `set_on(bool)` toggles visual, `is_on` property returns current state
    - _Requirements: 4.1, 4.2, 4.3_

  - [~] 2.2 Write property test for Light toggle visual correctness
    - **Property 2: Light toggle visual correctness**
    - Use `sampled_from(colors) × booleans()` to generate test cases
    - Verify on-state uses `color_on`, off-state uses dim dark color
    - **Validates: Requirements 4.1, 4.2**

- [ ] 3. Implement RocketCanvas widget
  - [~] 3.1 Implement `RocketCanvas(QGraphicsView)` with starfield, launch pad, and rocket drawing
    - Create `QGraphicsScene`, draw random star dots for starfield background
    - Draw launch pad structure at bottom of canvas
    - Draw rocket graphic: body, nose cone, window, fins — tag rocket items with `_is_rocket = True`
    - Initialize animation state: `_rocket_y=220`, `_flame_on=False`, `_launched=False`, `_launch_vel=0.0`
    - Set up `_timer: QTimer` with 40ms interval connected to `_animate` slot
    - _Requirements: 13.1, 13.2, 13.3, 13.4_

  - [~] 3.2 Implement `ignite()` method for launch animation
    - Set `_launched=True`, `_flame_on=True`, start the animation timer
    - `_animate` loop: increment velocity by 0.4, subtract velocity from `_rocket_y`, redraw rocket with flame polygons
    - Stop animation when `_rocket_y < -150` (off-screen)
    - _Requirements: 14.1, 14.2, 14.3_

  - [~] 3.3 Implement `reset()` method to return rocket to initial state
    - Stop animation timer, reset `_rocket_y=220`, `_launched=False`, `_flame_on=False`, `_launch_vel=0.0`
    - Redraw full scene (starfield, launch pad, rocket at baseline)
    - _Requirements: 14.4_

  - [~] 3.4 Write property test for rocket stationary in pre-launch states
    - **Property 6: Rocket stationary in pre-launch states**
    - Use `sampled_from(pre_launch_states)` to generate IDLE, CIRCUIT_TESTED, ENABLED, READY
    - Verify `rocket_y=220`, `launched=False`, `flame_on=False`
    - **Validates: Requirements 13.4**

  - [~] 3.5 Write property test for rocket animation velocity monotonically increasing
    - **Property 7: Rocket animation velocity is monotonically increasing**
    - Use `integers(min_value=2, max_value=50)` for frame count
    - Verify each frame: `launch_vel` increases, `rocket_y` decreases
    - **Validates: Requirements 14.2**

  - [~] 3.6 Write property test for rocket reset returns to initial state
    - **Property 8: Rocket reset returns to initial state**
    - Use `floats() × booleans() × booleans() × floats()` for arbitrary animation state
    - Verify after `reset()`: `rocket_y=220`, `launched=False`, `flame_on=False`, `launch_vel=0.0`, timer stopped
    - **Validates: Requirements 14.4**

- [~] 4. Checkpoint — Verify widgets
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 5. Implement RLSWindow main window and state machine
  - [~] 5.1 Implement `RLSWindow(QMainWindow)` constructor and `_build_ui` method
    - Set minimum window size 860×560, window title
    - Create LPU panel (left) with Test, Enable, Cancel buttons and green TEST / red ENABLE lights using `make_panel`, `make_button`, `Light`
    - Create CU panel (right) with Ready, Launch, Cancel buttons and red READY / green LAUNCH lights
    - Create `RocketCanvas` in center
    - Create state label below canvas
    - Create `QTextEdit` event log below state label, set read-only
    - Arrange layout: LPU left, canvas+label+log center, CU right using `QHBoxLayout` and `QVBoxLayout`
    - Initialize `_state = State.IDLE`, call `_update_ui()`
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 3.1, 3.2, 3.3, 5.1, 15.4_

  - [~] 5.2 Implement `_update_ui` method with button enable/disable matrix and light state matrix
    - Enable/disable all 6 buttons based on current `_state` per the design matrix
    - Set all 4 lights on/off based on current `_state` per the light state matrix
    - Update state label text to show current state name
    - _Requirements: 5.2, 5.3, 5.4, 6.4, 6.5, 7.3, 7.4, 8.4, 8.5, 9.5, 16.1_

  - [~] 5.3 Implement `_log(msg)` method for timestamped event logging
    - Append timestamped message to `QTextEdit` event log
    - Auto-scroll to most recent entry
    - _Requirements: 15.1, 15.2, 15.3_

  - [~] 5.4 Implement state transition handlers: `_on_test`, `_test_current_detected`, `_on_enable`
    - `_on_test`: Guard on `State.IDLE`, log button press, use `QTimer.singleShot(600ms)` to call `_test_current_detected`
    - `_test_current_detected`: Guard on `State.IDLE` (stale timer protection), set state to `CIRCUIT_TESTED`, log transition, call `_update_ui`
    - `_on_enable`: Guard on `State.CIRCUIT_TESTED`, set state to `ENABLED`, log transition, call `_update_ui`
    - _Requirements: 6.1, 6.2, 6.3, 7.1, 7.2, 11.1, 11.4, 12.1_

  - [~] 5.5 Implement state transition handlers: `_on_ready`, `_ready_acknowledged`, `_on_launch`, `_launch_acknowledged`
    - `_on_ready`: Guard on `State.ENABLED`, log button press, use `QTimer.singleShot(400ms)` to call `_ready_acknowledged`
    - `_ready_acknowledged`: Guard on `State.ENABLED`, set state to `READY`, log transition, call `_update_ui`
    - `_on_launch`: Guard on `State.READY`, log button press, use `QTimer.singleShot(500ms)` to call `_launch_acknowledged`
    - `_launch_acknowledged`: Guard on `State.READY`, set state to `LAUNCHED`, log transition, call `_update_ui`, call `canvas.ignite()`
    - _Requirements: 8.1, 8.2, 8.3, 9.1, 9.2, 9.3, 9.4, 11.2, 11.3, 12.2, 12.3_

  - [~] 5.6 Implement `_on_cancel` and `_reset` methods
    - `_on_cancel`: Determine which cancel button was pressed (LPU or CU), enforce CU cancel only valid in ENABLED/READY, log cancel event, call `_reset`
    - `_reset`: Set state to `IDLE`, turn off all 4 lights, reset canvas, call `_update_ui`, log reset
    - _Requirements: 10.1, 10.2, 10.3, 10.4_

  - [~] 5.7 Wire button signals to handler methods
    - Connect Test button `clicked` to `_on_test`
    - Connect Enable button `clicked` to `_on_enable`
    - Connect LPU Cancel button `clicked` to `_on_cancel`
    - Connect Ready button `clicked` to `_on_ready`
    - Connect Launch button `clicked` to `_on_launch`
    - Connect CU Cancel button `clicked` to `_on_cancel`
    - _Requirements: 6.1, 7.1, 8.1, 9.1, 10.1, 10.2_

- [~] 6. Checkpoint — Verify state machine and UI
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 7. Add application entry point and property-based tests for state machine
  - [ ] 7.1 Add `if __name__ == "__main__"` block to create `QApplication` and show `RLSWindow`
    - _Requirements: 1.1_

  - [ ] 7.2 Write property test for state label reflects current state
    - **Property 1: State label reflects current state**
    - Use `sampled_from(all_states)` to generate each valid state
    - Set `_state`, call `_update_ui()`, verify state label text contains state name
    - **Validates: Requirements 1.3**

  - [ ] 7.3 Write property test for button enable/disable matrix correctness
    - **Property 3: Button enable/disable matrix correctness**
    - Use `sampled_from(all_states)` to generate each valid state
    - Set `_state`, call `_update_ui()`, verify all 6 buttons match expected enable/disable per design matrix
    - **Validates: Requirements 5.2, 5.3, 5.4, 6.4, 6.5, 7.3, 7.4, 8.4, 8.5, 9.5, 16.1**

  - [ ] 7.4 Write property test for light state matrix correctness
    - **Property 4: Light state matrix correctness**
    - Use `sampled_from(all_states)` to generate each valid state
    - Set `_state`, call `_update_ui()`, verify all 4 lights match expected on/off per design matrix
    - **Validates: Requirements 6.3, 7.2, 8.3, 9.3, 10.3**

  - [ ] 7.5 Write property test for state transition map correctness
    - **Property 5: State transition map correctness**
    - Use `sampled_from(all_states) × sampled_from(all_buttons)` to generate state-button pairs
    - Invoke corresponding handler, verify resulting state matches `VALID_TRANSITIONS` map
    - **Validates: Requirements 6.2, 7.1, 8.2, 9.2, 10.1, 10.2, 11.1, 11.2, 11.3, 11.4**

  - [ ] 7.6 Write property test for event log grows on state transitions
    - **Property 9: Event log grows on state transitions**
    - Use `sampled_from(valid_transitions)` to generate valid state-transition pairs
    - Record log count before, execute transition, verify log count increased by at least one
    - **Validates: Requirements 15.1, 15.2**

- [ ] 8. Final checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design using Hypothesis
- Unit tests validate specific examples and edge cases
- The entire application is a single file (`rls_gui.py`) per the design's single-file architecture decision
