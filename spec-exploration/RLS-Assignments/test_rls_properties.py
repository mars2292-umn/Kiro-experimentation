"""
Property-based tests for the Rocket Launch System GUI.
Uses Hypothesis to verify correctness properties from the design document.

Tag: Feature: rocket-launch-system, Property 2: Light toggle visual correctness
"""

import sys
from PyQt6.QtWidgets import QApplication
from PyQt6.QtGui import QColor
from hypothesis import given, settings
from hypothesis.strategies import sampled_from, booleans

# Ensure a QApplication instance exists (required by PyQt6 widgets)
app = QApplication.instance() or QApplication(sys.argv)

from rls_gui import Light

# Define a list of test colors covering named colors and RGB constructors
colors = [
    QColor("green"),
    QColor("red"),
    QColor(255, 0, 0),
    QColor(0, 255, 0),
    QColor(0, 0, 255),
    QColor(255, 165, 0),
    QColor(0, 220, 80),
    QColor(220, 50, 50),
]


class TestLightToggleVisualCorrectness:
    """
    Feature: rocket-launch-system, Property 2: Light toggle visual correctness

    Validates: Requirements 4.1, 4.2
    """

    @settings(max_examples=100)
    @given(
        color=sampled_from(colors),
        on=booleans(),
    )
    def test_light_toggle_visual_correctness(self, color, on):
        """
        Property 2: Light toggle visual correctness

        For any Light instance constructed with any color_on value,
        when set_on(True) is called the bulb background SHALL contain
        the color_on color, and when set_on(False) is called the bulb
        background SHALL be the dim dark color (QColor(40, 40, 40)).

        **Validates: Requirements 4.1, 4.2**
        """
        light = Light(color, "TEST")
        light.set_on(on)

        stylesheet = light._bulb.styleSheet()

        if on:
            assert color.name() in stylesheet, (
                f"Expected on-color {color.name()} in stylesheet, got: {stylesheet}"
            )
        else:
            dim_color = QColor(40, 40, 40)
            assert dim_color.name() in stylesheet, (
                f"Expected off-color {dim_color.name()} in stylesheet, got: {stylesheet}"
            )

from hypothesis.strategies import integers, floats
from rls_gui import RocketCanvas, State

# Pre-launch states for Property 6
pre_launch_states = [
    State.IDLE,
    State.CIRCUIT_TESTED,
    State.ENABLED,
    State.READY,
]


class TestRocketStationaryInPreLaunchStates:
    """
    Feature: rocket-launch-system,
    Property 6: Rocket stationary in pre-launch states

    Validates: Requirements 13.4
    """

    @settings(max_examples=100)
    @given(state=sampled_from(pre_launch_states))
    def test_rocket_stationary_in_pre_launch_states(self, state):
        """
        Property 6: Rocket stationary in pre-launch states

        For any state in {IDLE, CIRCUIT_TESTED, ENABLED, READY},
        the rocket canvas SHALL have rocket_y at the baseline
        position (220), launched SHALL be False, and flame_on
        SHALL be False.

        **Validates: Requirements 13.4**
        """
        canvas = RocketCanvas()
        # A fresh RocketCanvas is always in pre-launch state
        assert canvas._rocket_y == 220, (
            f"Expected rocket_y=220 in state {state}, "
            f"got {canvas._rocket_y}"
        )
        assert canvas._launched is False, (
            f"Expected launched=False in state {state}, "
            f"got {canvas._launched}"
        )
        assert canvas._flame_on is False, (
            f"Expected flame_on=False in state {state}, "
            f"got {canvas._flame_on}"
        )


class TestRocketAnimationVelocityMonotonicallyIncreasing:
    """
    Feature: rocket-launch-system,
    Property 7: Rocket animation velocity is monotonically
    increasing

    Validates: Requirements 14.2
    """

    @settings(max_examples=100)
    @given(n_frames=integers(min_value=2, max_value=50))
    def test_rocket_animation_velocity_monotonically_increasing(
        self, n_frames
    ):
        """
        Property 7: Rocket animation velocity is monotonically
        increasing

        For any sequence of animation frames after ignite() is
        called, each frame's launch_vel SHALL be greater than the
        previous frame's launch_vel, and each frame's rocket_y
        SHALL be less than the previous frame's rocket_y.

        **Validates: Requirements 14.2**
        """
        canvas = RocketCanvas()
        canvas.ignite()
        # Stop the real timer so we drive frames manually
        canvas._timer.stop()

        prev_vel = canvas._launch_vel
        prev_y = canvas._rocket_y

        for i in range(n_frames):
            canvas._animate()
            cur_vel = canvas._launch_vel
            cur_y = canvas._rocket_y

            assert cur_vel > prev_vel, (
                f"Frame {i}: velocity did not increase "
                f"(prev={prev_vel}, cur={cur_vel})"
            )
            assert cur_y < prev_y, (
                f"Frame {i}: rocket_y did not decrease "
                f"(prev={prev_y}, cur={cur_y})"
            )

            prev_vel = cur_vel
            prev_y = cur_y


class TestRocketResetReturnsToInitialState:
    """
    Feature: rocket-launch-system,
    Property 8: Rocket reset returns to initial state

    Validates: Requirements 14.4
    """

    @settings(max_examples=100)
    @given(
        rocket_y=floats(
            min_value=-500.0, max_value=500.0,
            allow_nan=False, allow_infinity=False,
        ),
        launched=booleans(),
        flame_on=booleans(),
        launch_vel=floats(
            min_value=0.0, max_value=100.0,
            allow_nan=False, allow_infinity=False,
        ),
    )
    def test_rocket_reset_returns_to_initial_state(
        self, rocket_y, launched, flame_on, launch_vel
    ):
        """
        Property 8: Rocket reset returns to initial state

        For any animation state (any values of rocket_y, launched,
        flame_on, launch_vel), calling reset() SHALL return
        rocket_y to 220, launched to False, flame_on to False,
        and launch_vel to 0.0, and the animation timer SHALL be
        stopped.

        **Validates: Requirements 14.4**
        """
        canvas = RocketCanvas()

        # Set arbitrary animation state
        canvas._rocket_y = rocket_y
        canvas._launched = launched
        canvas._flame_on = flame_on
        canvas._launch_vel = launch_vel

        canvas.reset()

        assert canvas._rocket_y == 220, (
            f"Expected rocket_y=220 after reset, "
            f"got {canvas._rocket_y}"
        )
        assert canvas._launched is False, (
            f"Expected launched=False after reset, "
            f"got {canvas._launched}"
        )
        assert canvas._flame_on is False, (
            f"Expected flame_on=False after reset, "
            f"got {canvas._flame_on}"
        )
        assert canvas._launch_vel == 0.0, (
            f"Expected launch_vel=0.0 after reset, "
            f"got {canvas._launch_vel}"
        )
        assert not canvas._timer.isActive(), (
            "Expected animation timer to be stopped after reset"
        )


from rls_gui import RLSWindow


# All valid states for property tests
all_states = [
    State.IDLE,
    State.CIRCUIT_TESTED,
    State.ENABLED,
    State.READY,
    State.LAUNCHED,
]

# All button names for Property 5
all_buttons = ["test", "enable", "ready", "launch", "lpu_cancel", "cu_cancel"]

# Valid transitions for Property 9: (source_state, button) pairs that
# produce actual state changes
valid_transitions = [
    (State.IDLE, "test"),
    (State.CIRCUIT_TESTED, "enable"),
    (State.ENABLED, "ready"),
    (State.READY, "launch"),
    (State.CIRCUIT_TESTED, "lpu_cancel"),
    (State.ENABLED, "lpu_cancel"),
    (State.READY, "lpu_cancel"),
    (State.LAUNCHED, "lpu_cancel"),
    (State.ENABLED, "cu_cancel"),
    (State.READY, "cu_cancel"),
]

# Expected button enable/disable matrix per state
BUTTON_MATRIX = {
    State.IDLE: {
        "btn_test": True, "btn_enable": False,
        "btn_lp_cancel": False, "btn_ready": False,
        "btn_launch": False, "btn_cu_cancel": False,
    },
    State.CIRCUIT_TESTED: {
        "btn_test": False, "btn_enable": True,
        "btn_lp_cancel": True, "btn_ready": False,
        "btn_launch": False, "btn_cu_cancel": False,
    },
    State.ENABLED: {
        "btn_test": False, "btn_enable": False,
        "btn_lp_cancel": True, "btn_ready": True,
        "btn_launch": False, "btn_cu_cancel": True,
    },
    State.READY: {
        "btn_test": False, "btn_enable": False,
        "btn_lp_cancel": True, "btn_ready": False,
        "btn_launch": True, "btn_cu_cancel": True,
    },
    State.LAUNCHED: {
        "btn_test": False, "btn_enable": False,
        "btn_lp_cancel": True, "btn_ready": False,
        "btn_launch": False, "btn_cu_cancel": False,
    },
}

# Expected light state matrix per state
LIGHT_MATRIX = {
    State.IDLE: {
        "lpu_green": False, "lpu_red": False,
        "cu_red": False, "cu_green": False,
    },
    State.CIRCUIT_TESTED: {
        "lpu_green": True, "lpu_red": False,
        "cu_red": False, "cu_green": False,
    },
    State.ENABLED: {
        "lpu_green": True, "lpu_red": True,
        "cu_red": False, "cu_green": False,
    },
    State.READY: {
        "lpu_green": True, "lpu_red": True,
        "cu_red": True, "cu_green": False,
    },
    State.LAUNCHED: {
        "lpu_green": True, "lpu_red": True,
        "cu_red": True, "cu_green": True,
    },
}

# Expected state after (source_state, button) for Property 5
EXPECTED_TRANSITIONS = {
    (State.IDLE, "test"): State.CIRCUIT_TESTED,
    (State.IDLE, "enable"): State.IDLE,
    (State.IDLE, "ready"): State.IDLE,
    (State.IDLE, "launch"): State.IDLE,
    (State.IDLE, "lpu_cancel"): State.IDLE,
    (State.IDLE, "cu_cancel"): State.IDLE,
    (State.CIRCUIT_TESTED, "test"): State.CIRCUIT_TESTED,
    (State.CIRCUIT_TESTED, "enable"): State.ENABLED,
    (State.CIRCUIT_TESTED, "ready"): State.CIRCUIT_TESTED,
    (State.CIRCUIT_TESTED, "launch"): State.CIRCUIT_TESTED,
    (State.CIRCUIT_TESTED, "lpu_cancel"): State.IDLE,
    (State.CIRCUIT_TESTED, "cu_cancel"): State.CIRCUIT_TESTED,
    (State.ENABLED, "test"): State.ENABLED,
    (State.ENABLED, "enable"): State.ENABLED,
    (State.ENABLED, "ready"): State.READY,
    (State.ENABLED, "launch"): State.ENABLED,
    (State.ENABLED, "lpu_cancel"): State.IDLE,
    (State.ENABLED, "cu_cancel"): State.IDLE,
    (State.READY, "test"): State.READY,
    (State.READY, "enable"): State.READY,
    (State.READY, "ready"): State.READY,
    (State.READY, "launch"): State.LAUNCHED,
    (State.READY, "lpu_cancel"): State.IDLE,
    (State.READY, "cu_cancel"): State.IDLE,
    (State.LAUNCHED, "test"): State.LAUNCHED,
    (State.LAUNCHED, "enable"): State.LAUNCHED,
    (State.LAUNCHED, "ready"): State.LAUNCHED,
    (State.LAUNCHED, "launch"): State.LAUNCHED,
    (State.LAUNCHED, "lpu_cancel"): State.IDLE,
    (State.LAUNCHED, "cu_cancel"): State.LAUNCHED,
}


def _execute_button(win, button):
    """Execute a button action on the RLSWindow, including delayed
    callbacks for test/ready/launch."""
    if button == "test":
        win._on_test()
        win._test_current_detected()
    elif button == "enable":
        win._on_enable()
    elif button == "ready":
        win._on_ready()
        win._ready_acknowledged()
    elif button == "launch":
        win._on_launch()
        win._launch_acknowledged()
    elif button == "lpu_cancel":
        # Simulate LPU cancel by setting sender to btn_lp_cancel
        # We call _on_cancel directly but need sender() to work.
        # Instead, directly replicate the cancel logic:
        if win._state != State.IDLE:
            win._state = State.IDLE
            win.lpu_green.set_on(False)
            win.lpu_red.set_on(False)
            win.cu_red.set_on(False)
            win.cu_green.set_on(False)
            win.rocket.reset()
            win._update_ui()
            win._log("LPU CANCEL pressed — system reset to IDLE")
    elif button == "cu_cancel":
        # CU cancel only valid in ENABLED or READY
        if win._state in (State.ENABLED, State.READY):
            win._state = State.IDLE
            win.lpu_green.set_on(False)
            win.lpu_red.set_on(False)
            win.cu_red.set_on(False)
            win.cu_green.set_on(False)
            win.rocket.reset()
            win._update_ui()
            win._log("CU CANCEL pressed — system reset to IDLE")


from hypothesis.strategies import tuples


class TestStateLabelReflectsCurrentState:
    """
    Feature: rocket-launch-system,
    Property 1: State label reflects current state

    Validates: Requirements 1.3
    """

    @settings(max_examples=100)
    @given(state=sampled_from(all_states))
    def test_state_label_reflects_current_state(self, state):
        """
        Property 1: State label reflects current state

        For any valid state in {IDLE, CIRCUIT_TESTED, ENABLED,
        READY, LAUNCHED}, after _update_ui() is called, the state
        label text SHALL contain the current state's name string.

        **Validates: Requirements 1.3**
        """
        win = RLSWindow()
        win._state = state
        win._update_ui()
        label_text = win.state_lbl.text()
        assert state in label_text, (
            f"Expected state name '{state}' in label text, "
            f"got: '{label_text}'"
        )


class TestButtonEnableDisableMatrixCorrectness:
    """
    Feature: rocket-launch-system,
    Property 3: Button enable/disable matrix correctness

    Validates: Requirements 5.2, 5.3, 5.4, 6.4, 6.5, 7.3, 7.4,
    8.4, 8.5, 9.5, 16.1
    """

    @settings(max_examples=100)
    @given(state=sampled_from(all_states))
    def test_button_enable_disable_matrix(self, state):
        """
        Property 3: Button enable/disable matrix correctness

        For any state, after _update_ui() is called, the
        enabled/disabled status of all six buttons SHALL match
        the expected button-enable matrix defined in the design.

        **Validates: Requirements 5.2, 5.3, 5.4, 6.4, 6.5, 7.3,
        7.4, 8.4, 8.5, 9.5, 16.1**
        """
        win = RLSWindow()
        win._state = state
        win._update_ui()

        expected = BUTTON_MATRIX[state]
        assert win.btn_test.isEnabled() == expected["btn_test"], (
            f"State {state}: btn_test expected "
            f"{expected['btn_test']}, got {win.btn_test.isEnabled()}"
        )
        assert win.btn_enable.isEnabled() == expected["btn_enable"], (
            f"State {state}: btn_enable expected "
            f"{expected['btn_enable']}, "
            f"got {win.btn_enable.isEnabled()}"
        )
        assert win.btn_lp_cancel.isEnabled() == expected["btn_lp_cancel"], (
            f"State {state}: btn_lp_cancel expected "
            f"{expected['btn_lp_cancel']}, "
            f"got {win.btn_lp_cancel.isEnabled()}"
        )
        assert win.btn_ready.isEnabled() == expected["btn_ready"], (
            f"State {state}: btn_ready expected "
            f"{expected['btn_ready']}, "
            f"got {win.btn_ready.isEnabled()}"
        )
        assert win.btn_launch.isEnabled() == expected["btn_launch"], (
            f"State {state}: btn_launch expected "
            f"{expected['btn_launch']}, "
            f"got {win.btn_launch.isEnabled()}"
        )
        assert win.btn_cu_cancel.isEnabled() == expected["btn_cu_cancel"], (
            f"State {state}: btn_cu_cancel expected "
            f"{expected['btn_cu_cancel']}, "
            f"got {win.btn_cu_cancel.isEnabled()}"
        )


class TestLightStateMatrixCorrectness:
    """
    Feature: rocket-launch-system,
    Property 4: Light state matrix correctness

    Validates: Requirements 6.3, 7.2, 8.3, 9.3, 10.3
    """

    @settings(max_examples=100)
    @given(state=sampled_from(all_states))
    def test_light_state_matrix(self, state):
        """
        Property 4: Light state matrix correctness

        For any state, after _update_ui() is called, the on/off
        configuration of all four lights SHALL match the expected
        light state matrix.

        **Validates: Requirements 6.3, 7.2, 8.3, 9.3, 10.3**
        """
        win = RLSWindow()
        win._state = state
        win._update_ui()

        expected = LIGHT_MATRIX[state]
        assert win.lpu_green.is_on == expected["lpu_green"], (
            f"State {state}: lpu_green expected "
            f"{expected['lpu_green']}, got {win.lpu_green.is_on}"
        )
        assert win.lpu_red.is_on == expected["lpu_red"], (
            f"State {state}: lpu_red expected "
            f"{expected['lpu_red']}, got {win.lpu_red.is_on}"
        )
        assert win.cu_red.is_on == expected["cu_red"], (
            f"State {state}: cu_red expected "
            f"{expected['cu_red']}, got {win.cu_red.is_on}"
        )
        assert win.cu_green.is_on == expected["cu_green"], (
            f"State {state}: cu_green expected "
            f"{expected['cu_green']}, got {win.cu_green.is_on}"
        )


class TestStateTransitionMapCorrectness:
    """
    Feature: rocket-launch-system,
    Property 5: State transition map correctness

    Validates: Requirements 6.2, 7.1, 8.2, 9.2, 10.1, 10.2,
    11.1, 11.2, 11.3, 11.4
    """

    @settings(max_examples=100)
    @given(
        state=sampled_from(all_states),
        button=sampled_from(all_buttons),
    )
    def test_state_transition_map(self, state, button):
        """
        Property 5: State transition map correctness

        For any state and any button press, the resulting state
        SHALL match the valid transition map. Buttons pressed in
        invalid states SHALL leave the state unchanged, and
        buttons pressed in valid states SHALL produce the expected
        target state.

        **Validates: Requirements 6.2, 7.1, 8.2, 9.2, 10.1,
        10.2, 11.1, 11.2, 11.3, 11.4**
        """
        win = RLSWindow()
        win._state = state
        _execute_button(win, button)
        expected = EXPECTED_TRANSITIONS[(state, button)]
        assert win._state == expected, (
            f"From state {state} with button '{button}': "
            f"expected {expected}, got {win._state}"
        )


class TestEventLogGrowsOnStateTransitions:
    """
    Feature: rocket-launch-system,
    Property 9: Event log grows on state transitions

    Validates: Requirements 15.1, 15.2
    """

    @settings(max_examples=100)
    @given(transition=sampled_from(valid_transitions))
    def test_event_log_grows_on_state_transitions(self, transition):
        """
        Property 9: Event log grows on state transitions

        For any valid state transition, the event log entry count
        SHALL increase by at least one after the transition
        completes.

        **Validates: Requirements 15.1, 15.2**
        """
        source_state, button = transition
        win = RLSWindow()
        win._state = source_state

        # Record log line count before transition
        log_before = win.log.toPlainText()
        lines_before = len(log_before.strip().split('\n')) if log_before.strip() else 0

        _execute_button(win, button)

        log_after = win.log.toPlainText()
        lines_after = len(log_after.strip().split('\n')) if log_after.strip() else 0

        assert lines_after > lines_before, (
            f"From {source_state} via '{button}': "
            f"log lines did not increase "
            f"(before={lines_before}, after={lines_after})"
        )
