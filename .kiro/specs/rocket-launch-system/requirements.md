# Requirements Document

## Introduction

The Rocket Launch System (RLS) GUI Simulator is a Python desktop application built with PyQt6 that simulates a two-unit rocket launch system. The system models a Launch Pad Unit (LPU) and a Control Unit (CU) that communicate wirelessly to safely test, arm, and launch a model rocket. The GUI provides interactive buttons, visual indicator lights, a rocket animation, an event log, and enforces a strict state machine that governs the launch sequence. The application is intended as an educational tool for demonstrating embedded systems concepts including state machines, event-driven design, and safety-critical sequencing.

## Glossary

- **RLS_Simulator**: The top-level Python GUI application that hosts both unit panels, the rocket canvas, and the event log.
- **Launch_Pad_Unit (LPU)**: The left panel of the GUI representing the physical launch pad hardware. Contains Test, Enable, and Cancel buttons, plus a green Test light and a red Enable light.
- **Control_Unit (CU)**: The right panel of the GUI representing the handheld control unit. Contains Ready, Launch, and Cancel buttons, plus a red Ready light and a green Launch light.
- **State_Machine**: The internal finite state machine that governs all transitions between system states and enforces proper sequencing of operations.
- **Rocket_Canvas**: The central visual area that renders a rocket graphic on a launch pad with a starfield background, and animates the rocket launch sequence upon successful ignition.
- **Event_Log**: A read-only text area that displays timestamped messages describing each state transition and system event.
- **Light_Indicator**: A circular GUI widget that visually represents an on/off indicator by changing between a dim off-state color and a bright on-state color.
- **Test_Circuit**: The low-current circuit path used to verify battery charge and igniter wiring integrity before enabling the system.
- **Igniter_Circuit**: The high-current circuit path that, when closed, sends ignition current to the rocket motor.

## Requirements

### Requirement 1: Application Window and Layout

**User Story:** As a user, I want the RLS Simulator to display both the Launch Pad Unit and Control Unit side by side with a rocket visual in the center, so that I can observe and interact with the full system from a single window.

#### Acceptance Criteria

1. THE RLS_Simulator SHALL display a main window with a minimum size of 860 by 560 pixels.
2. THE RLS_Simulator SHALL arrange the Launch_Pad_Unit panel on the left, the Rocket_Canvas in the center, and the Control_Unit panel on the right in a horizontal layout.
3. THE RLS_Simulator SHALL display the current State_Machine state as a text label below the Rocket_Canvas.
4. THE RLS_Simulator SHALL display the Event_Log below the Rocket_Canvas and state label.

### Requirement 2: Launch Pad Unit Panel

**User Story:** As a user, I want the Launch Pad Unit panel to contain the correct buttons and lights, so that I can interact with the LPU side of the system.

#### Acceptance Criteria

1. THE Launch_Pad_Unit SHALL display a Test button, an Enable button, and a Cancel button.
2. THE Launch_Pad_Unit SHALL display a green Light_Indicator labeled "TEST" and a red Light_Indicator labeled "ENABLE".
3. WHEN the RLS_Simulator starts, THE Launch_Pad_Unit SHALL display both Light_Indicators in the off state.

### Requirement 3: Control Unit Panel

**User Story:** As a user, I want the Control Unit panel to contain the correct buttons and lights, so that I can interact with the CU side of the system.

#### Acceptance Criteria

1. THE Control_Unit SHALL display a Ready button, a Launch button, and a Cancel button.
2. THE Control_Unit SHALL display a red Light_Indicator labeled "READY" and a green Light_Indicator labeled "LAUNCH".
3. WHEN the RLS_Simulator starts, THE Control_Unit SHALL display both Light_Indicators in the off state.

### Requirement 4: Light Indicator Widget

**User Story:** As a user, I want indicator lights that visually change between on and off states, so that I can clearly see the current status of each indicator.

#### Acceptance Criteria

1. WHEN a Light_Indicator is in the off state, THE Light_Indicator SHALL render as a dim dark circle.
2. WHEN a Light_Indicator is in the on state, THE Light_Indicator SHALL render as a bright circle in the designated color (green or red).
3. THE Light_Indicator SHALL display a text label beneath the circle identifying the indicator purpose.

### Requirement 5: State Machine — Initial State

**User Story:** As a user, I want the system to start in a safe idle state, so that no accidental actions can occur before I begin the launch sequence.

#### Acceptance Criteria

1. WHEN the RLS_Simulator starts, THE State_Machine SHALL be in the IDLE state.
2. WHILE the State_Machine is in the IDLE state, THE RLS_Simulator SHALL enable only the Test button on the Launch_Pad_Unit.
3. WHILE the State_Machine is in the IDLE state, THE RLS_Simulator SHALL disable the Enable button, Ready button, and Launch button.
4. WHILE the State_Machine is in the IDLE state, THE RLS_Simulator SHALL disable both Cancel buttons.

### Requirement 6: State Machine — Test Circuit Transition

**User Story:** As a user, I want to press the Test button to verify the battery and igniter circuit, so that I can confirm the hardware is functioning before enabling the system.

#### Acceptance Criteria

1. WHEN the Test button is pressed WHILE the State_Machine is in the IDLE state, THE State_Machine SHALL initiate a test circuit check.
2. WHEN the test circuit check completes successfully, THE State_Machine SHALL transition to the CIRCUIT_TESTED state.
3. WHEN the State_Machine transitions to the CIRCUIT_TESTED state, THE Launch_Pad_Unit SHALL turn on the green TEST Light_Indicator.
4. WHILE the State_Machine is in the CIRCUIT_TESTED state, THE RLS_Simulator SHALL enable the Enable button and the Launch_Pad_Unit Cancel button.
5. WHILE the State_Machine is in the CIRCUIT_TESTED state, THE RLS_Simulator SHALL disable the Test button, Ready button, and Launch button.

### Requirement 7: State Machine — Enable Transition

**User Story:** As a user, I want to press the Enable button to arm the launch pad and enable wireless communication, so that the Control Unit can connect to the Launch Pad Unit.

#### Acceptance Criteria

1. WHEN the Enable button is pressed WHILE the State_Machine is in the CIRCUIT_TESTED state, THE State_Machine SHALL transition to the ENABLED state.
2. WHEN the State_Machine transitions to the ENABLED state, THE Launch_Pad_Unit SHALL turn on the red ENABLE Light_Indicator.
3. WHILE the State_Machine is in the ENABLED state, THE RLS_Simulator SHALL enable the Ready button on the Control_Unit and both Cancel buttons.
4. WHILE the State_Machine is in the ENABLED state, THE RLS_Simulator SHALL disable the Test button, Enable button, and Launch button.

### Requirement 8: State Machine — Ready Transition

**User Story:** As a user, I want to press the Ready button on the Control Unit to establish a connection with the Launch Pad Unit, so that I can confirm the wireless link before launching.

#### Acceptance Criteria

1. WHEN the Ready button is pressed WHILE the State_Machine is in the ENABLED state, THE State_Machine SHALL send a ready notification to the Launch_Pad_Unit.
2. WHEN the Launch_Pad_Unit acknowledges the ready notification, THE State_Machine SHALL transition to the READY state.
3. WHEN the State_Machine transitions to the READY state, THE Control_Unit SHALL turn on the red READY Light_Indicator.
4. WHILE the State_Machine is in the READY state, THE RLS_Simulator SHALL enable the Launch button and both Cancel buttons.
5. WHILE the State_Machine is in the READY state, THE RLS_Simulator SHALL disable the Test button, Enable button, and Ready button.

### Requirement 9: State Machine — Launch Transition

**User Story:** As a user, I want to press the Launch button to ignite the rocket, so that I can observe a successful launch sequence.

#### Acceptance Criteria

1. WHEN the Launch button is pressed WHILE the State_Machine is in the READY state, THE State_Machine SHALL send a launch command to the Launch_Pad_Unit.
2. WHEN the Launch_Pad_Unit acknowledges the launch command, THE State_Machine SHALL transition to the LAUNCHED state.
3. WHEN the State_Machine transitions to the LAUNCHED state, THE Control_Unit SHALL turn on the green LAUNCH Light_Indicator.
4. WHEN the State_Machine transitions to the LAUNCHED state, THE Rocket_Canvas SHALL begin the rocket launch animation.
5. WHILE the State_Machine is in the LAUNCHED state, THE RLS_Simulator SHALL disable all buttons except the Launch_Pad_Unit Cancel button.

### Requirement 10: State Machine — Cancel and Reset

**User Story:** As a user, I want to cancel the launch sequence from either unit at any time, so that I can safely abort and return the system to its initial state.

#### Acceptance Criteria

1. WHEN the Launch_Pad_Unit Cancel button is pressed WHILE the State_Machine is not in the IDLE state, THE State_Machine SHALL transition to the IDLE state.
2. WHEN the Control_Unit Cancel button is pressed WHILE the State_Machine is not in the IDLE or LAUNCHED state, THE State_Machine SHALL transition to the IDLE state.
3. WHEN the State_Machine transitions to the IDLE state via cancel, THE RLS_Simulator SHALL turn off all four Light_Indicators.
4. WHEN the State_Machine transitions to the IDLE state via cancel, THE Rocket_Canvas SHALL reset to the initial pre-launch visual.

### Requirement 11: State Machine — Sequencing Enforcement

**User Story:** As a user, I want the system to enforce proper sequencing so that I cannot skip steps in the launch procedure.

#### Acceptance Criteria

1. THE State_Machine SHALL reject the Enable button press WHILE the State_Machine is not in the CIRCUIT_TESTED state.
2. THE State_Machine SHALL reject the Ready button press WHILE the State_Machine is not in the ENABLED state.
3. THE State_Machine SHALL reject the Launch button press WHILE the State_Machine is not in the READY state.
4. THE State_Machine SHALL reject the Test button press WHILE the State_Machine is not in the IDLE state.

### Requirement 12: Simulated Communication Delays

**User Story:** As a user, I want the system to simulate realistic communication delays between units, so that the GUI feels like a real wireless system rather than instant state changes.

#### Acceptance Criteria

1. WHEN the Test button is pressed, THE RLS_Simulator SHALL delay the test current detection response by a configurable duration before transitioning state.
2. WHEN the Ready button is pressed, THE RLS_Simulator SHALL delay the Launch_Pad_Unit acknowledgment by a configurable duration before transitioning state.
3. WHEN the Launch button is pressed, THE RLS_Simulator SHALL delay the Launch_Pad_Unit acknowledgment by a configurable duration before transitioning state.

### Requirement 13: Rocket Canvas Visual

**User Story:** As a user, I want to see a rocket graphic on a launch pad with a starfield background, so that the simulation feels visually engaging.

#### Acceptance Criteria

1. THE Rocket_Canvas SHALL display a starfield background with randomly placed star dots.
2. THE Rocket_Canvas SHALL display a launch pad structure at the bottom of the canvas.
3. THE Rocket_Canvas SHALL display a rocket graphic consisting of a body, nose cone, window, and fins resting on the launch pad.
4. WHILE the State_Machine is in the IDLE, CIRCUIT_TESTED, ENABLED, or READY state, THE Rocket_Canvas SHALL display the rocket in a stationary position on the launch pad.

### Requirement 14: Rocket Launch Animation

**User Story:** As a user, I want to see the rocket animate upward with flame effects when launch succeeds, so that I get clear visual feedback of a successful ignition.

#### Acceptance Criteria

1. WHEN the Rocket_Canvas ignite method is called, THE Rocket_Canvas SHALL display animated flame polygons beneath the rocket.
2. WHEN the Rocket_Canvas ignite method is called, THE Rocket_Canvas SHALL animate the rocket moving upward with increasing velocity.
3. WHEN the rocket moves above the visible canvas area, THE Rocket_Canvas SHALL stop the animation.
4. WHEN the Rocket_Canvas reset method is called, THE Rocket_Canvas SHALL return the rocket to the stationary position on the launch pad and remove flame effects.

### Requirement 15: Event Log

**User Story:** As a user, I want to see a log of all system events, so that I can trace the sequence of actions and diagnose issues.

#### Acceptance Criteria

1. WHEN a state transition occurs, THE Event_Log SHALL append a descriptive message identifying the transition.
2. WHEN a button is pressed, THE Event_Log SHALL append a message identifying the button press event.
3. THE Event_Log SHALL automatically scroll to display the most recent message.
4. THE Event_Log SHALL be read-only so that the user cannot modify log entries.

### Requirement 16: Button State Management

**User Story:** As a user, I want buttons to be enabled only when their action is valid in the current state, so that I receive clear guidance on what actions are available.

#### Acceptance Criteria

1. WHEN the State_Machine transitions to a new state, THE RLS_Simulator SHALL update the enabled or disabled status of all buttons to reflect the valid actions for the new state.
2. WHILE a button is disabled, THE RLS_Simulator SHALL render the button with a visually distinct disabled appearance.
3. WHILE a button is enabled, THE RLS_Simulator SHALL render the button with its designated active color scheme.
