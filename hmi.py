#!/usr/bin/env python3
from dataclasses import dataclass
from typing import Dict
import can
import cantools
import cantools.database


from textual import on
from textual.app import App, ComposeResult
from textual.widgets import Label
from textual_slider import Slider
from textual.widget import Widget
from textual.containers import Horizontal


class CANScheduler:
    @dataclass
    class Entry:
        msg: cantools.database.Database
        signals: Dict
        cyclic_handle: can.broadcastmanager.CyclicSendTaskABC = None

        def encode(self) -> can.Message:
            return can.Message(
                arbitration_id=self.msg.frame_id,
                data=self.msg.encode(self.signals),
                is_fd=self.msg.is_fd,
                is_extended_id=self.msg.is_extended_frame,
            )

    def __init__(self, bus: can.interface.Bus, db: cantools.database.Database):
        self.db = db
        self.bus = bus
        self.messages: Dict[str, CANScheduler.Entry] = {}

    def add_message(self, name):
        msg = db.get_message_by_name(name)
        signals = {s.name: s.initial or 0 for s in msg.signals}

        entry = self.messages[name] = CANScheduler.Entry(msg, signals)
        entry.cyclic_handle = bus.send_periodic(entry.encode(), msg.cycle_time or 0.1)

    def update(self, frame, signal, value):
        entry = self.messages.get(frame)
        if not entry:
            self.add_message(frame)
            entry = self.messages.get(frame)

        if signal not in entry.signals:
            raise KeyError(f"Unknown signal {signal} in {frame}")
        entry.signals[signal] = value
        entry.cyclic_handle.modify_data(entry.encode())


class WheelsWidget(Widget):
    DEFAULT_CSS = """
    WheelsWidget * {
        padding-top: 1;
    }
    WheelsWidget Label {
        width: 10;
    }
    """

    def compose(self) -> ComposeResult:
        with Horizontal():
            yield Label("Wheels")
            yield Slider(-45, 45, value=0)

    @on(Slider.Changed)
    def on_slider_changed(self):
        value = self.query_one(Slider).value
        self.query_one(Label).update(f"Wheels {value}")
        can_scheduler.update("WHEEL_ANGLE", "Wheel_Angle", value)


class TestApp(App):
    def compose(self) -> ComposeResult:
        yield WheelsWidget()


if __name__ == "__main__":
    db = cantools.database.load_file("STM_BUS.dbc")
    with can.interface.Bus(interface="socketcan", channel="can0", fd=False) as bus:
        can_scheduler = CANScheduler(bus, db)

        app = TestApp()
        app.run()
