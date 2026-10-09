#!/usr/bin/env python3
"""
Rocket Launch System (RLS) Simulator
Based on SEng 5801 Homework Assignments 1–5 (Heimdahl)

Implements the UML state machine defined in HW5 with the full
input/output event interface from the assignment.

Requires: PyQt6  (pip install PyQt6)
"""
import sys
import datetime
import random
from enum import Enum

from PyQt6.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout, QHBoxLayout,
    QGridLayout, QLabel, QPushButton, QTextEdit, QGroupBox,
    QCheckBox,
)
from PyQt6.QtCore import Qt, QTimer, QPointF
from PyQt6.QtGui import (
    QPainter, QColor, QBrush, QPen, QFont, QPolygonF, QPalette,
)

# ── Fixed star positions (so the pad view doesn't flicker on redraws) ─────────
_STARS = [(random.randint(3, 167), random.randint(3, 195)) for _ in range(55)]


# ── State machine states (HW5) ─────────────────────────────────────────────────
class State(Enum):
    IDLE      = "Idle"
    TESTING   = "Testing Circuit"
    TEST_PASS = "Test Passed"
    ENABLED   = "Launch Pad Enabled"
    READY     = "Ready to Launch"
    LAUNCHING = "Launching"
    LAUNCHED  = "Launched!"


# ── LED-style indicator widget ─────────────────────────────────────────────────
class LED(QWidget):
    COLORS = {
        "off":   (QColor("#1a1a1a"), QColor("#444444")),
        "green": (QColor("#00ff55"), QColor("#004422")),
        "red":   (QColor("#ff3333"), QColor("#550000")),
    }

    def __init__(self, label: str, parent=None):
        super().__init__(parent)
        self._color = "off"
        layout = QVBoxLayout(self)
        layout.setContentsMargins(4, 4, 4, 4)
        layout.setSpacing(2)

        self._canvas = _LEDCanvas(self)
        lbl = QLabel(label)
        lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
        lbl.setStyleSheet("color:#888888; font-size:8pt;")

        layout.addWidget(self._canvas, alignment=Qt.AlignmentFlag.AlignCenter)
        layout.addWidget(lbl)

    def set_color(self, color: str):
        self._color = color
        self._canvas.set_color(color)


class _LEDCanvas(QWidget):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setFixedSize(38, 38)
        self._color = "off"

    def set_color(self, color: str):
        self._color = color
        self.update()

    def paintEvent(self, _):
        fill, outline = LED.COLORS.get(self._color, LED.COLORS["off"])
        p = QPainter(self)
        p.setRenderHint(QPainter.RenderHint.Antialiasing)
        p.setBrush(QBrush(fill))
        p.setPen(QPen(outline, 2))
        p.drawEllipse(4, 4, 30, 30)


# ── Launch pad canvas ──────────────────────────────────────────────────────────
class PadCanvas(QWidget):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setFixedSize(180, 265)
        self._state = State.IDLE
        self._igniter_connected = False
        self._flame = False

    def update_state(self, state: State, igniter: bool, flame: bool = False):
        self._state = state
        self._igniter_connected = igniter
        self._flame = flame
        self.update()

    def paintEvent(self, _):
        p = QPainter(self)
        p.setRenderHint(QPainter.RenderHint.Antialiasing)
        s = self._state
        w, h = self.width(), self.height()

        # Sky
        p.fillRect(0, 0, w, 230, QColor("#020210"))

        # Stars
        p.setPen(QPen(QColor("white"), 1))
        for sx, sy in _STARS:
            p.drawPoint(sx, sy)

        # Ground
        p.fillRect(0, 228, w, h - 228, QColor("#1a120a"))
        p.fillRect(0, 224, w, 5, QColor("#2a1e10"))

        if s == State.LAUNCHED:
            p.setPen(QPen(QColor("#ffaa00")))
            p.setFont(QFont("Helvetica", 13, QFont.Weight.Bold))
            p.drawText(0, 0, w, 230, Qt.AlignmentFlag.AlignCenter, "LAUNCHED!\nIgnition Successful")
            # Empty stand
            p.fillRect(82, 185, 16, 43, QColor("#555555"))
            p.fillRect(68, 222, 44, 8, QColor("#666666"))
            return

        # Launch stand
        p.fillRect(82, 170, 16, 58, QColor("#555555"))
        p.fillRect(68, 222, 44, 8, QColor("#666666"))

        rx, ry_base = 90, 222
        body_top = ry_base - 95

        # Rocket body
        p.setBrush(QBrush(QColor("#d0d0d0")))
        p.setPen(QPen(QColor("#aaaaaa"), 1))
        p.drawRect(rx - 13, body_top + 16, 26, 95)

        # Porthole
        p.setBrush(QBrush(QColor("#88ccff")))
        p.setPen(QPen(QColor("#aaddff"), 1))
        p.drawEllipse(rx - 6, body_top + 28, 12, 12)

        # Nose cone
        nose = QPolygonF([
            QPointF(rx - 13, body_top + 16),
            QPointF(rx + 13, body_top + 16),
            QPointF(rx, body_top - 16),
        ])
        p.setBrush(QBrush(QColor("#cc3333")))
        p.setPen(QPen(QColor("#aa2222"), 1))
        p.drawPolygon(nose)

        # Fins
        fin_l = QPolygonF([
            QPointF(rx - 13, ry_base - 10),
            QPointF(rx - 30, ry_base + 10),
            QPointF(rx - 13, ry_base - 30),
        ])
        fin_r = QPolygonF([
            QPointF(rx + 13, ry_base - 10),
            QPointF(rx + 30, ry_base + 10),
            QPointF(rx + 13, ry_base - 30),
        ])
        p.setBrush(QBrush(QColor("#888888")))
        p.setPen(QPen(QColor("#aaaaaa"), 1))
        p.drawPolygon(fin_l)
        p.drawPolygon(fin_r)

        # Igniter wires
        if self._igniter_connected:
            wc = (
                QColor("#00ff44")
                if s in (State.TEST_PASS, State.ENABLED, State.READY, State.LAUNCHING)
                else QColor("#ffdd00")
            )
            p.setPen(QPen(wc, 2))
            p.drawPolyline(QPolygonF([
                QPointF(rx - 13, ry_base - 14),
                QPointF(rx - 22, ry_base + 2),
                QPointF(rx - 18, ry_base + 14),
            ]))
            p.drawPolyline(QPolygonF([
                QPointF(rx + 13, ry_base - 14),
                QPointF(rx + 22, ry_base + 2),
                QPointF(rx + 18, ry_base + 14),
            ]))

        # Engine flames
        if self._flame or s == State.LAUNCHING:
            outer = QPolygonF([
                QPointF(rx - 9, ry_base - 10),
                QPointF(rx + 9, ry_base - 10),
                QPointF(rx + 5, ry_base + 24),
                QPointF(rx, ry_base + 42),
                QPointF(rx - 5, ry_base + 24),
            ])
            inner = QPolygonF([
                QPointF(rx - 5, ry_base - 10),
                QPointF(rx + 5, ry_base - 10),
                QPointF(rx, ry_base + 22),
            ])
            p.setBrush(QBrush(QColor("#ff6600")))
            p.setPen(Qt.PenStyle.NoPen)
            p.drawPolygon(outer)
            p.setBrush(QBrush(QColor("#ffee00")))
            p.drawPolygon(inner)


# ── Main window ────────────────────────────────────────────────────────────────
class RLSApp(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("Rocket Launch System Simulator — SEng 5801")
        self.setMinimumSize(860, 700)

        # Model
        self.state = State.IDLE
        self.igniter_connected = False
        self.circuit_intact = True

        self._build_ui()
        self._refresh()
        self._log("System initialised.  Connect igniter to LPU to begin.", "INFO")

    # ── UI construction ────────────────────────────────────────────────────────

    def _build_ui(self):
        central = QWidget()
        self.setCentralWidget(central)
        root = QVBoxLayout(central)
        root.setContentsMargins(10, 10, 10, 10)
        root.setSpacing(8)

        # Title
        title = QLabel("Rocket Launch System Simulator")
        title.setAlignment(Qt.AlignmentFlag.AlignCenter)
        title.setStyleSheet("font-size:18pt; font-weight:bold; color:white; padding:6px;")
        root.addWidget(title)

        # State banner
        self.banner = QLabel()
        self.banner.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.banner.setStyleSheet(
            "font-size:12pt; font-weight:bold; padding:6px; border-radius:4px;"
        )
        root.addWidget(self.banner)

        # 3-column content
        cols = QHBoxLayout()
        cols.setSpacing(8)
        root.addLayout(cols, stretch=1)

        cols.addWidget(self._build_lpu(),      stretch=1)
        cols.addWidget(self._build_pad_view(), stretch=0)
        cols.addWidget(self._build_cu(),       stretch=1)

        # Simulation controls
        sim_box = QGroupBox("Simulation Controls")
        sim_box.setStyleSheet(
            "QGroupBox { color:#888888; font-size:9pt; border:1px solid #333344;"
            " border-radius:4px; margin-top:6px; padding:4px; }"
            "QGroupBox::title { subcontrol-origin:margin; left:8px; }"
        )
        sim_row = QHBoxLayout(sim_box)

        self.circuit_cb = QCheckBox("Igniter circuit physically intact  "
                                    "(uncheck to simulate a broken wire)")
        self.circuit_cb.setChecked(True)
        self.circuit_cb.setStyleSheet("color:#aaaaaa; font-size:9pt;")
        self.circuit_cb.toggled.connect(self._on_circuit_toggle)
        sim_row.addWidget(self.circuit_cb)
        sim_row.addStretch()

        self.btn_reset = QPushButton("Full Reset")
        self.btn_reset.setStyleSheet(
            "QPushButton { background:#3a2a00; color:#ffcc44; padding:4px 12px;"
            " border-radius:3px; font-size:9pt; }"
            "QPushButton:hover { background:#5a4200; }"
        )
        self.btn_reset.clicked.connect(self._full_reset)
        sim_row.addWidget(self.btn_reset)
        root.addWidget(sim_box)

        # Event log
        log_box = QGroupBox("Event Log")
        log_box.setStyleSheet(
            "QGroupBox { color:#666688; font-size:9pt; border:1px solid #333344;"
            " border-radius:4px; margin-top:6px; padding:4px; }"
            "QGroupBox::title { subcontrol-origin:margin; left:8px; }"
        )
        log_layout = QVBoxLayout(log_box)
        self.log_view = QTextEdit()
        self.log_view.setReadOnly(True)
        self.log_view.setStyleSheet(
            "background:#050510; color:#00cc55; font-family:Courier; font-size:9pt;"
            " border:none;"
        )
        self.log_view.setFixedHeight(180)
        log_layout.addWidget(self.log_view)
        root.addWidget(log_box)

        self.setStyleSheet("QMainWindow, QWidget { background:#0d0d1a; }")

    def _make_btn(self, text: str, slot, bg: str, w: int = 110) -> QPushButton:
        btn = QPushButton(text)
        btn.setFixedSize(w, 46)
        btn.setFont(QFont("Helvetica", 10, QFont.Weight.Bold))
        btn.setStyleSheet(
            f"QPushButton {{ background:{bg}; color:white; border-radius:4px; }}"
            f"QPushButton:hover {{ background:{bg}cc; }}"
            f"QPushButton:disabled {{ background:#2a2a3a; color:#555566; }}"
        )
        btn.clicked.connect(slot)
        return btn

    def _build_lpu(self) -> QGroupBox:
        box = QGroupBox(" Launch Pad Unit (LPU) ")
        box.setStyleSheet(
            "QGroupBox { color:#55cc55; font-size:11pt; font-weight:bold;"
            " border:2px solid #1c5a1c; border-radius:6px;"
            " margin-top:10px; padding:8px; background:#0a160a; }"
            "QGroupBox::title { subcontrol-origin:margin; left:10px; }"
        )
        lay = QVBoxLayout(box)
        lay.setSpacing(10)

        # Igniter connector
        ig_lbl = QLabel("Igniter Connector")
        ig_lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
        ig_lbl.setStyleSheet("color:#888888; font-size:9pt;")
        lay.addWidget(ig_lbl)

        self.btn_igniter = QPushButton("Connect Igniter")
        self.btn_igniter.setFixedHeight(32)
        self.btn_igniter.setStyleSheet(
            "QPushButton { background:#1c4a1c; color:white; border-radius:3px; font-size:9pt; }"
            "QPushButton:hover { background:#2a6a2a; }"
            "QPushButton:disabled { background:#2a2a3a; color:#555566; }"
        )
        self.btn_igniter.clicked.connect(self._toggle_igniter)
        lay.addWidget(self.btn_igniter)

        # Lights
        lights = QHBoxLayout()
        lights.addStretch()
        self.lpu_green = LED("TEST (green)")
        self.lpu_red   = LED("ENABLE (red)")
        lights.addWidget(self.lpu_green)
        lights.addSpacing(20)
        lights.addWidget(self.lpu_red)
        lights.addStretch()
        lay.addLayout(lights)

        # Buttons
        btns = QGridLayout()
        btns.setSpacing(6)
        self.btn_test        = self._make_btn("TEST",         self._ev_test_pressed,   "#1c5a1c")
        self.btn_enable      = self._make_btn("ENABLE",       self._ev_enable_pressed, "#5a1c1c")
        self.btn_lp_cancel   = self._make_btn("CANCEL (LP)",  self._ev_lp_cancel,      "#5a4a00", w=230)
        btns.addWidget(self.btn_test,      0, 0)
        btns.addWidget(self.btn_enable,    0, 1)
        btns.addWidget(self.btn_lp_cancel, 1, 0, 1, 2)
        lay.addLayout(btns)
        lay.addStretch()
        return box

    def _build_cu(self) -> QGroupBox:
        box = QGroupBox(" Control Unit (CU) ")
        box.setStyleSheet(
            "QGroupBox { color:#5555cc; font-size:11pt; font-weight:bold;"
            " border:2px solid #1c1c5a; border-radius:6px;"
            " margin-top:10px; padding:8px; background:#0a0a1a; }"
            "QGroupBox::title { subcontrol-origin:margin; left:10px; }"
        )
        lay = QVBoxLayout(box)
        lay.setSpacing(10)

        wireless = QLabel("〰  Wireless link  〰")
        wireless.setAlignment(Qt.AlignmentFlag.AlignCenter)
        wireless.setStyleSheet("color:#333366; font-size:8pt; font-style:italic;")
        lay.addWidget(wireless)

        lights = QHBoxLayout()
        lights.addStretch()
        self.cu_red   = LED("READY (red)")
        self.cu_green = LED("LAUNCH (green)")
        lights.addWidget(self.cu_red)
        lights.addSpacing(20)
        lights.addWidget(self.cu_green)
        lights.addStretch()
        lay.addLayout(lights)

        btns = QGridLayout()
        btns.setSpacing(6)
        self.btn_ready     = self._make_btn("READY",        self._ev_ready_pressed,  "#5a1c1c")
        self.btn_launch    = self._make_btn("LAUNCH",       self._ev_launch_pressed, "#8a0000")
        self.btn_cu_cancel = self._make_btn("CANCEL (CU)",  self._ev_cu_cancel,      "#5a4a00", w=230)
        btns.addWidget(self.btn_ready,     0, 0)
        btns.addWidget(self.btn_launch,    0, 1)
        btns.addWidget(self.btn_cu_cancel, 1, 0, 1, 2)
        lay.addLayout(btns)
        lay.addStretch()
        return box

    def _build_pad_view(self) -> QGroupBox:
        box = QGroupBox(" Launch Pad View ")
        box.setStyleSheet(
            "QGroupBox { color:#666666; font-size:11pt; font-weight:bold;"
            " border:2px solid #333333; border-radius:6px;"
            " margin-top:10px; padding:8px; background:#0d0d0d; }"
            "QGroupBox::title { subcontrol-origin:margin; left:10px; }"
        )
        lay = QVBoxLayout(box)
        self.pad = PadCanvas()
        lay.addWidget(self.pad)
        return box

    # ── Input event handlers (HW5 interface) ──────────────────────────────────

    def _toggle_igniter(self):
        if self.state != State.IDLE:
            self._log("Igniter change blocked — system is active.", "WARN")
            return
        self.igniter_connected = not self.igniter_connected
        if self.igniter_connected:
            self.btn_igniter.setText("Disconnect Igniter")
            self.btn_igniter.setStyleSheet(
                "QPushButton { background:#6a2020; color:white; border-radius:3px; font-size:9pt; }"
                "QPushButton:hover { background:#8a3030; }"
            )
            self._log("Igniter wired to LPU connectors.  Circuit closed externally.", "EVENT")
        else:
            self.btn_igniter.setText("Connect Igniter")
            self.btn_igniter.setStyleSheet(
                "QPushButton { background:#1c4a1c; color:white; border-radius:3px; font-size:9pt; }"
                "QPushButton:hover { background:#2a6a2a; }"
                "QPushButton:disabled { background:#2a2a3a; color:#555566; }"
            )
            self._log("Igniter disconnected from LPU.", "EVENT")
        self._refresh()

    def _on_circuit_toggle(self, checked: bool):
        self.circuit_intact = checked
        self._log(
            f"[SIM] Physical igniter circuit set to: {'intact' if checked else 'BROKEN'}",
            "INFO" if checked else "WARN",
        )

    def _ev_test_pressed(self):
        if self.state != State.IDLE:
            self._log("Test Button ignored — not in IDLE state.", "WARN")
            return
        if not self.igniter_connected:
            self._log("Test Button Pressed — ERROR: igniter not connected.", "ERROR")
            return
        self._log("INPUT  EVENT: Test Button Pressed", "EVENT")
        self._log("OUTPUT EVENT: Close Test Circuit — sending low-current pulse…", "EVENT")
        self._goto(State.TESTING)
        QTimer.singleShot(1400, self._test_current_result)

    def _test_current_result(self):
        if self.state != State.TESTING:
            return
        if self.circuit_intact:
            self._log("INPUT  EVENT: Test Current Detected", "EVENT")
            self._log("OUTPUT EVENT: Test Light On  (green light on LPU)", "EVENT")
            self.lpu_green.set_color("green")
            self._goto(State.TEST_PASS)
        else:
            self._log("INPUT  EVENT: (no test current — circuit broken)", "ERROR")
            self._log("Circuit check FAILED.  Check wiring and retry.", "ERROR")
            self._goto(State.IDLE)

    def _ev_enable_pressed(self):
        if self.state != State.TEST_PASS:
            self._log("Enable Button ignored — circuit must be tested first.", "WARN")
            return
        self._log("INPUT  EVENT: Enable Button Pressed", "EVENT")
        self._log("OUTPUT EVENT: LP Enable Light On  (red light on LPU)", "EVENT")
        self.lpu_red.set_color("red")
        self._goto(State.ENABLED)

    def _ev_ready_pressed(self):
        if self.state != State.ENABLED:
            self._log("Ready Button ignored — LPU must be enabled first.", "WARN")
            return
        self._log("INPUT  EVENT: Ready Button Pressed  (CU)", "EVENT")
        self._log("OUTPUT EVENT: CU Enable Light On  (red light on CU — link established)", "EVENT")
        self.cu_red.set_color("red")
        self._goto(State.READY)

    def _ev_launch_pressed(self):
        if self.state != State.READY:
            self._log("Launch Button ignored — not in READY state.", "WARN")
            return
        self._log("INPUT  EVENT: Launch Button Pressed  (CU)", "EVENT")
        self._log("OUTPUT EVENT: Close Igniter Connection — firing igniter circuit…", "EVENT")
        self._goto(State.LAUNCHING)
        QTimer.singleShot(2200, self._launch_complete)

    def _launch_complete(self):
        if self.state != State.LAUNCHING:
            return
        self._log("INPUT  EVENT: Launch Successful", "EVENT")
        self._log("OUTPUT EVENT: Launch Light On  (green light on CU)", "EVENT")
        self.cu_green.set_color("green")
        self._goto(State.LAUNCHED)

    def _ev_lp_cancel(self):
        if self.state in (State.IDLE, State.LAUNCHED, State.LAUNCHING):
            self._log("LP Cancel: nothing to abort in current state.", "WARN")
            return
        self._log("INPUT EVENT: Launch Pad Cancel Pressed → resetting to IDLE.", "WARN")
        self._full_reset()

    def _ev_cu_cancel(self):
        if self.state in (State.IDLE, State.LAUNCHED, State.LAUNCHING):
            self._log("CU Cancel: nothing to abort in current state.", "WARN")
            return
        self._log("INPUT EVENT: Control Unit Cancel Pressed → resetting to IDLE.", "WARN")
        self._full_reset()

    # ── State machine ──────────────────────────────────────────────────────────

    def _goto(self, new_state: State):
        old = self.state
        self.state = new_state
        self._log(f"Transition:  {old.value}  →  {new_state.value}", "STATE")
        self._refresh()

    def _full_reset(self):
        self.state = State.IDLE
        self.igniter_connected = False
        self.circuit_cb.setChecked(True)
        self.circuit_intact = True
        self.btn_igniter.setText("Connect Igniter")
        self.btn_igniter.setStyleSheet(
            "QPushButton { background:#1c4a1c; color:white; border-radius:3px; font-size:9pt; }"
            "QPushButton:hover { background:#2a6a2a; }"
            "QPushButton:disabled { background:#2a2a3a; color:#555566; }"
        )
        for led in (self.lpu_green, self.lpu_red, self.cu_red, self.cu_green):
            led.set_color("off")
        self._refresh()
        self._log("System reset to initial IDLE state.", "STATE")

    # ── UI refresh ─────────────────────────────────────────────────────────────

    def _refresh(self):
        s = self.state
        cfg = {
            State.IDLE:      ("#1e1e00", "#cccc44",
                              f"STATE: {s.value.upper()}   —   Connect igniter to begin"),
            State.TESTING:   ("#002200", "#44ff66",
                              f"STATE: {s.value.upper()}   —   Testing circuit…"),
            State.TEST_PASS: ("#003300", "#00ff77",
                              f"STATE: {s.value.upper()}   —   Press ENABLE when ready"),
            State.ENABLED:   ("#330000", "#ff7777",
                              f"STATE: {s.value.upper()}   —   Walk to safe distance, then press READY"),
            State.READY:     ("#440000", "#ff4444",
                              f"STATE: {s.value.upper()}   —   Press LAUNCH to ignite"),
            State.LAUNCHING: ("#550000", "#ff2222",
                              f"STATE: {s.value.upper()}   —   Ignition sequence in progress…"),
            State.LAUNCHED:  ("#003322", "#00ffaa",
                              f"STATE: {s.value.upper()}   —   SUCCESS"),
        }
        bg, fg, text = cfg.get(s, ("#111111", "#ffffff", s.value))
        self.banner.setText(text)
        self.banner.setStyleSheet(
            f"font-size:12pt; font-weight:bold; padding:6px; border-radius:4px;"
            f" background:{bg}; color:{fg};"
        )

        # Update pad canvas
        self.pad.update_state(s, self.igniter_connected, flame=(s == State.LAUNCHING))

        # Button enable/disable
        idle      = s == State.IDLE
        t_pass    = s == State.TEST_PASS
        enabled   = s == State.ENABLED
        ready     = s == State.READY
        terminal  = s in (State.LAUNCHING, State.LAUNCHED)
        cancellable = not idle and not terminal

        self.btn_igniter.setEnabled(idle)
        self.btn_test.setEnabled(idle and self.igniter_connected)
        self.btn_enable.setEnabled(t_pass)
        self.btn_ready.setEnabled(enabled)
        self.btn_launch.setEnabled(ready)
        self.btn_lp_cancel.setEnabled(cancellable)
        self.btn_cu_cancel.setEnabled(cancellable)

    # ── Logging ────────────────────────────────────────────────────────────────

    def _log(self, msg: str, level: str = "INFO"):
        ts = datetime.datetime.now().strftime("%H:%M:%S")
        colors = {
            "INFO":  "#00cc55",
            "WARN":  "#ffaa00",
            "ERROR": "#ff4455",
            "EVENT": "#44aaff",
            "STATE": "#cc88ff",
        }
        color = colors.get(level, "#00cc55")
        html = (
            f'<span style="color:{color}; font-family:Courier; font-size:9pt;">'
            f"[{ts}]  {msg}</span><br>"
        )
        self.log_view.moveCursor(self.log_view.textCursor().MoveOperation.End)
        self.log_view.insertHtml(html)
        self.log_view.moveCursor(self.log_view.textCursor().MoveOperation.End)


# ── Entry point ────────────────────────────────────────────────────────────────
def main():
    app = QApplication(sys.argv)
    app.setStyle("Fusion")
    # Dark palette base
    palette = QPalette()
    palette.setColor(QPalette.ColorRole.Window,          QColor("#0d0d1a"))
    palette.setColor(QPalette.ColorRole.WindowText,      QColor("white"))
    palette.setColor(QPalette.ColorRole.Base,            QColor("#050510"))
    palette.setColor(QPalette.ColorRole.AlternateBase,   QColor("#111122"))
    palette.setColor(QPalette.ColorRole.Text,            QColor("white"))
    palette.setColor(QPalette.ColorRole.ButtonText,      QColor("white"))
    palette.setColor(QPalette.ColorRole.Button,          QColor("#222233"))
    app.setPalette(palette)

    win = RLSApp()
    win.show()
    sys.exit(app.exec())


if __name__ == "__main__":
    main()
