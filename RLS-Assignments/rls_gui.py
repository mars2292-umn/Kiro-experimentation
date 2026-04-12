"""
Rocket Launch System (RLS) GUI
Implements the full state machine from HW-5 (SEng 5801)

State Machine:
  IDLE
    -> TEST_BUTTON_PRESSED  => close test circuit, check connection
    -> TEST_CURRENT_DETECTED => Test Light ON
    -> ENABLE_BUTTON_PRESSED => LP Enable Light ON, comms enabled
    -> READY_BUTTON_PRESSED  => CU Enable Light ON
    -> LAUNCH_BUTTON_PRESSED => Close Igniter, Launch Light ON, rocket launches
    -> CANCEL (either unit)  => reset to IDLE

Two panels:
  Launch Pad Unit (LPU): Test btn, Enable btn, Cancel btn, Green light (test), Red light (enable)
  Control Unit (CU):     Ready btn, Launch btn, Cancel btn, Red light (CU enable), Green light (launch)
"""

import sys
from PyQt6.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout, QHBoxLayout,
    QPushButton, QLabel, QFrame, QGraphicsView, QGraphicsScene,
    QGraphicsEllipseItem, QGraphicsRectItem, QGraphicsPolygonItem,
    QGraphicsLineItem, QTextEdit, QSizePolicy, QGraphicsItem
)
from PyQt6.QtCore import Qt, QTimer, QPointF, QRectF, pyqtSignal
from PyQt6.QtGui import (
    QColor, QPainter, QBrush, QPen, QPolygonF, QFont, QLinearGradient,
    QRadialGradient, QPainterPath
)

# ── State machine states ──────────────────────────────────────────────────────
class State:
    IDLE            = "IDLE"
    CIRCUIT_TESTED  = "CIRCUIT_TESTED"   # test current detected, green light on
    ENABLED         = "ENABLED"          # enable pressed, LP red light on
    READY           = "READY"            # ready pressed, CU red light on
    LAUNCHED        = "LAUNCHED"         # launch confirmed, rocket fires


# ── Light widget ──────────────────────────────────────────────────────────────
class Light(QWidget):
    def __init__(self, color_on: QColor, label: str, parent=None):
        super().__init__(parent)
        self._color_on  = color_on
        self._color_off = QColor(40, 40, 40)
        self._on = False
        self.setFixedSize(54, 70)

        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(2)

        self._bulb = QLabel()
        self._bulb.setFixedSize(44, 44)
        self._bulb.setAlignment(Qt.AlignmentFlag.AlignCenter)

        lbl = QLabel(label)
        lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
        lbl.setStyleSheet("color: #ccc; font-size: 10px;")

        layout.addWidget(self._bulb, alignment=Qt.AlignmentFlag.AlignHCenter)
        layout.addWidget(lbl)
        self._refresh()

    def _refresh(self):
        c = self._color_on if self._on else self._color_off
        glow = ""
        if self._on:
            glow = f"box-shadow: 0 0 12px {c.name()};"
        self._bulb.setStyleSheet(
            f"background: {c.name()}; border-radius: 22px; border: 2px solid #555; {glow}"
        )

    def set_on(self, on: bool):
        self._on = on
        self._refresh()

    @property
    def is_on(self):
        return self._on


# ── Rocket canvas ─────────────────────────────────────────────────────────────
class RocketCanvas(QGraphicsView):
    def __init__(self, parent=None):
        super().__init__(parent)
        self._scene = QGraphicsScene(self)
        self.setScene(self._scene)
        self.setFixedSize(200, 340)
        self.setStyleSheet("background: #0a0a1a; border: 1px solid #333; border-radius: 8px;")
        self.setRenderHint(QPainter.RenderHint.Antialiasing)
        self.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        self.setVerticalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)

        self._rocket_y   = 220   # baseline y of rocket body bottom
        self._flame_on   = False
        self._launched   = False
        self._launch_vel = 0.0

        self._timer = QTimer(self)
        self._timer.timeout.connect(self._animate)

        self._draw()

    # ── drawing ──
    def _draw(self):
        self._scene.clear()
        W, H = 200, 340

        # stars
        import random
        random.seed(42)
        for _ in range(60):
            x, y = random.randint(0, W), random.randint(0, H)
            r = random.uniform(0.5, 1.5)
            star = self._scene.addEllipse(x, y, r*2, r*2,
                QPen(Qt.PenStyle.NoPen), QBrush(QColor(255, 255, 255, random.randint(80, 200))))

        # launch pad
        pad = self._scene.addRect(60, 295, 80, 8,
            QPen(QColor(100, 100, 100)), QBrush(QColor(80, 80, 80)))
        leg_l = self._scene.addLine(75, 295, 65, 310,
            QPen(QColor(120, 120, 120), 3))
        leg_r = self._scene.addLine(125, 295, 135, 310,
            QPen(QColor(120, 120, 120), 3))

        self._draw_rocket()

    def _draw_rocket(self):
        # remove old rocket items
        for item in list(self._scene.items()):
            if getattr(item, '_is_rocket', False):
                self._scene.removeItem(item)

        y = self._rocket_y
        cx = 100

        def tag(item):
            item._is_rocket = True
            return item

        # body
        body = tag(self._scene.addRect(cx - 14, y - 80, 28, 80,
            QPen(Qt.PenStyle.NoPen), QBrush(QColor(200, 210, 220))))

        # nose cone (triangle)
        nose = QPolygonF([
            QPointF(cx, y - 120),
            QPointF(cx - 14, y - 80),
            QPointF(cx + 14, y - 80),
        ])
        nose_item = tag(self._scene.addPolygon(nose,
            QPen(Qt.PenStyle.NoPen), QBrush(QColor(220, 60, 60))))

        # window
        win = tag(self._scene.addEllipse(cx - 7, y - 65, 14, 14,
            QPen(QColor(100, 180, 255), 2), QBrush(QColor(30, 100, 180, 180))))

        # fins
        fin_l = QPolygonF([
            QPointF(cx - 14, y - 20),
            QPointF(cx - 30, y),
            QPointF(cx - 14, y),
        ])
        fin_r = QPolygonF([
            QPointF(cx + 14, y - 20),
            QPointF(cx + 30, y),
            QPointF(cx + 14, y),
        ])
        tag(self._scene.addPolygon(fin_l, QPen(Qt.PenStyle.NoPen), QBrush(QColor(180, 60, 60))))
        tag(self._scene.addPolygon(fin_r, QPen(Qt.PenStyle.NoPen), QBrush(QColor(180, 60, 60))))

        # flame
        if self._flame_on:
            import random
            flicker = random.randint(-4, 4)
            flame1 = QPolygonF([
                QPointF(cx - 10, y),
                QPointF(cx + 10, y),
                QPointF(cx + flicker, y + 45),
            ])
            flame2 = QPolygonF([
                QPointF(cx - 6, y),
                QPointF(cx + 6, y),
                QPointF(cx + flicker // 2, y + 28),
            ])
            tag(self._scene.addPolygon(flame1, QPen(Qt.PenStyle.NoPen),
                QBrush(QColor(255, 140, 0, 200))))
            tag(self._scene.addPolygon(flame2, QPen(Qt.PenStyle.NoPen),
                QBrush(QColor(255, 240, 80, 220))))

    # ── animation ──
    def _animate(self):
        if self._launched:
            self._launch_vel += 0.4
            self._rocket_y -= self._launch_vel
            self._flame_on = True
            if self._rocket_y < -150:
                self._timer.stop()
                self._flame_on = False
        else:
            # idle flicker
            self._flame_on = not self._flame_on

        self._draw_rocket()

    def ignite(self):
        self._launched = True
        self._launch_vel = 1.0
        self._timer.start(40)

    def reset(self):
        self._rocket_y   = 220
        self._launched   = False
        self._flame_on   = False
        self._launch_vel = 0.0
        self._timer.stop()
        self._draw()


# ── Panel base ────────────────────────────────────────────────────────────────
def make_panel(title: str, color: str) -> tuple[QFrame, QVBoxLayout]:
    frame = QFrame()
    frame.setStyleSheet(f"""
        QFrame {{
            background: #1e1e2e;
            border: 2px solid {color};
            border-radius: 10px;
        }}
    """)
    layout = QVBoxLayout(frame)
    layout.setSpacing(10)
    layout.setContentsMargins(16, 12, 16, 16)

    title_lbl = QLabel(title)
    title_lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
    title_lbl.setStyleSheet(f"color: {color}; font-size: 14px; font-weight: bold; border: none;")
    layout.addWidget(title_lbl)

    sep = QFrame()
    sep.setFrameShape(QFrame.Shape.HLine)
    sep.setStyleSheet(f"color: {color}; border: 1px solid {color};")
    layout.addWidget(sep)

    return frame, layout


def make_button(label: str, color: str) -> QPushButton:
    btn = QPushButton(label)
    btn.setFixedHeight(38)
    btn.setStyleSheet(f"""
        QPushButton {{
            background: #2a2a3e;
            color: {color};
            border: 2px solid {color};
            border-radius: 6px;
            font-size: 12px;
            font-weight: bold;
        }}
        QPushButton:hover {{
            background: {color};
            color: #0a0a1a;
        }}
        QPushButton:disabled {{
            background: #1a1a2a;
            color: #444;
            border-color: #333;
        }}
    """)
    return btn


# ── Main window ───────────────────────────────────────────────────────────────
class RLSWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("Rocket Launch System — RLS Simulator")
        self.setStyleSheet("background: #0d0d1a; color: #eee;")
        self.setMinimumSize(860, 560)

        self._state = State.IDLE
        self._build_ui()
        self._update_ui()

    # ── UI construction ──
    def _build_ui(self):
        root = QWidget()
        self.setCentralWidget(root)
        main = QHBoxLayout(root)
        main.setSpacing(16)
        main.setContentsMargins(16, 16, 16, 16)

        # ── Launch Pad Unit ──
        lpu_frame, lpu = make_panel("🚀  LAUNCH PAD UNIT", "#ff6b6b")

        lights_row = QHBoxLayout()
        self.lpu_green = Light(QColor(0, 220, 80),  "TEST")
        self.lpu_red   = Light(QColor(220, 50, 50), "ENABLE")
        lights_row.addStretch()
        lights_row.addWidget(self.lpu_green)
        lights_row.addSpacing(20)
        lights_row.addWidget(self.lpu_red)
        lights_row.addStretch()
        lpu.addLayout(lights_row)

        self.btn_test   = make_button("TEST",   "#00dc50")
        self.btn_enable = make_button("ENABLE", "#ff6b6b")
        self.btn_lp_cancel = make_button("CANCEL", "#888")

        lpu.addWidget(self.btn_test)
        lpu.addWidget(self.btn_enable)
        lpu.addStretch()
        lpu.addWidget(self.btn_lp_cancel)

        # ── Rocket canvas (center) ──
        self.rocket = RocketCanvas()
        center = QVBoxLayout()
        center.addStretch()
        center.addWidget(self.rocket, alignment=Qt.AlignmentFlag.AlignHCenter)
        center.addStretch()

        # ── Control Unit ──
        cu_frame, cu = make_panel("🎮  CONTROL UNIT", "#6bcbff")

        lights_row2 = QHBoxLayout()
        self.cu_red   = Light(QColor(220, 50, 50),  "READY")
        self.cu_green = Light(QColor(0, 220, 80),   "LAUNCH")
        lights_row2.addStretch()
        lights_row2.addWidget(self.cu_red)
        lights_row2.addSpacing(20)
        lights_row2.addWidget(self.cu_green)
        lights_row2.addStretch()
        cu.addLayout(lights_row2)

        self.btn_ready  = make_button("READY",  "#6bcbff")
        self.btn_launch = make_button("LAUNCH 🔥", "#ffcc00")
        self.btn_cu_cancel = make_button("CANCEL", "#888")

        cu.addWidget(self.btn_ready)
        cu.addWidget(self.btn_launch)
        cu.addStretch()
        cu.addWidget(self.btn_cu_cancel)

        # ── Log ──
        log_frame = QFrame()
        log_frame.setStyleSheet("background: #111122; border: 1px solid #333; border-radius: 6px;")
        log_layout = QVBoxLayout(log_frame)
        log_layout.setContentsMargins(8, 6, 8, 6)
        log_title = QLabel("EVENT LOG")
        log_title.setStyleSheet("color: #888; font-size: 10px; font-weight: bold; border: none;")
        self.log = QTextEdit()
        self.log.setReadOnly(True)
        self.log.setFixedHeight(90)
        self.log.setStyleSheet("background: transparent; color: #aaffaa; font-size: 11px; border: none; font-family: monospace;")
        log_layout.addWidget(log_title)
        log_layout.addWidget(self.log)

        # ── State label ──
        self.state_lbl = QLabel(f"STATE: {self._state}")
        self.state_lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.state_lbl.setStyleSheet("color: #ffcc00; font-size: 12px; font-weight: bold;")

        # ── Assemble ──
        left_col = QVBoxLayout()
        left_col.addWidget(lpu_frame)

        right_col = QVBoxLayout()
        right_col.addWidget(cu_frame)

        mid_col = QVBoxLayout()
        mid_col.addLayout(center)
        mid_col.addWidget(self.state_lbl)
        mid_col.addWidget(log_frame)

        main.addLayout(left_col, 2)
        main.addLayout(mid_col, 3)
        main.addLayout(right_col, 2)

        # ── Wire buttons ──
        self.btn_test.clicked.connect(self._on_test)
        self.btn_enable.clicked.connect(self._on_enable)
        self.btn_lp_cancel.clicked.connect(self._on_cancel)
        self.btn_ready.clicked.connect(self._on_ready)
        self.btn_launch.clicked.connect(self._on_launch)
        self.btn_cu_cancel.clicked.connect(self._on_cancel)

    # ── State machine transitions ──
    def _on_test(self):
        if self._state == State.IDLE:
            self._log("TEST button pressed → closing test circuit…")
            # simulate test current detected after short delay
            QTimer.singleShot(600, self._test_current_detected)

    def _test_current_detected(self):
        if self._state == State.IDLE:
            self._log("✅ Test current detected — circuit OK")
            self._state = State.CIRCUIT_TESTED
            self.lpu_green.set_on(True)   # Test Light ON
            self._update_ui()

    def _on_enable(self):
        if self._state == State.CIRCUIT_TESTED:
            self._log("ENABLE button pressed → LP Enable Light ON, comms enabled")
            self._state = State.ENABLED
            self.lpu_red.set_on(True)     # LP Enable Light ON
            self._update_ui()

    def _on_ready(self):
        if self._state == State.ENABLED:
            self._log("READY button pressed → notifying launch pad…")
            QTimer.singleShot(400, self._ready_acknowledged)

    def _ready_acknowledged(self):
        if self._state == State.ENABLED:
            self._log("✅ Launch pad acknowledged READY — CU Enable Light ON")
            self._state = State.READY
            self.cu_red.set_on(True)      # CU Enable Light ON
            self._update_ui()

    def _on_launch(self):
        if self._state == State.READY:
            self._log("LAUNCH button pressed → sending launch command…")
            QTimer.singleShot(500, self._launch_acknowledged)

    def _launch_acknowledged(self):
        if self._state == State.READY:
            self._log("🔥 Launch pad acknowledged — closing igniter circuit!")
            self._state = State.LAUNCHED
            self.cu_green.set_on(True)    # Launch Light ON
            self._update_ui()
            self.rocket.ignite()
            self._log("🚀 ROCKET LAUNCHED!")

    def _on_cancel(self):
        if self._state != State.IDLE:
            self._log("⛔ CANCEL pressed — system reset to IDLE")
        self._reset()

    def _reset(self):
        self._state = State.IDLE
        self.lpu_green.set_on(False)
        self.lpu_red.set_on(False)
        self.cu_red.set_on(False)
        self.cu_green.set_on(False)
        self.rocket.reset()
        self._update_ui()

    # ── Enable/disable buttons per state ──
    def _update_ui(self):
        s = self._state
        self.state_lbl.setText(f"STATE:  {s}")

        # LPU buttons
        self.btn_test.setEnabled(s == State.IDLE)
        self.btn_enable.setEnabled(s == State.CIRCUIT_TESTED)
        self.btn_lp_cancel.setEnabled(s != State.IDLE)

        # CU buttons — only available once LP is enabled
        self.btn_ready.setEnabled(s == State.ENABLED)
        self.btn_launch.setEnabled(s == State.READY)
        self.btn_cu_cancel.setEnabled(s not in (State.IDLE, State.LAUNCHED))

    def _log(self, msg: str):
        self.log.append(msg)
        self.log.verticalScrollBar().setValue(
            self.log.verticalScrollBar().maximum()
        )


# ── Entry point ───────────────────────────────────────────────────────────────
if __name__ == "__main__":
    app = QApplication(sys.argv)
    app.setFont(QFont("Helvetica Neue", 11))
    win = RLSWindow()
    win.show()
    sys.exit(app.exec())
