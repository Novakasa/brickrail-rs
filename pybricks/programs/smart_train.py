from micropython import const
from ustruct import pack, pack_into

from pybricks.pupdevices import ColorDistanceSensor, DCMotor, Motor
from pybricks.parameters import Port

from io_hub_unfrozen import IOHub, VERSION

# version 0

_COLOR_YELLOW = const(0)
_COLOR_BLUE = const(1)
_COLOR_GREEN = const(2)
_COLOR_RED = const(3)
_COLOR_ANY = const(15)
COLOR_HUES = (51, 219, 133, 359)

_SENSOR_KEY_NONE = const(0)
_SENSOR_KEY_ENTER = const(1)
_SENSOR_KEY_IN = const(2)

_SENSOR_SPEED_FAST = const(1)
_SENSOR_SPEED_SLOW = const(2)
_SENSOR_SPEED_CRUISE = const(3)

_LEG_FLAG_BACKWARDS = const(1)
_LEG_FLAG_STOP = const(2)

_STATE_FLAG_STOP = const(32)
_STATE_FLAG_RUN = const(64)
_STATE_FLAG_BACKWARDS = const(128)

_DATA_ROUTE_COMPLETE = const(1)
_DATA_LEG_ADVANCE = const(2)
_DATA_SENSOR_ADVANCE = const(3)
_DATA_UNEXPECTED_MARKER = const(4)

_CONFIG_CHROMA_THRESHOLD = const(0)
_CONFIG_MOTOR_ACC = const(1)
_CONFIG_MOTOR_DEC = const(2)
_CONFIG_MOTOR_FAST_SPEED = const(3)
_CONFIG_MOTOR_SLOW_SPEED = const(4)
_CONFIG_MOTOR_CRUISE_SPEED = const(5)
_CONFIG_MOTOR_INVERTED = const(6)  # and the following 5 adresses are also reserved

_DUMP_TYPE_COLORS = const(1)


class TrainSensor:
    def __init__(self, marker_exit_callback):
        for port in ["A", "B", "C", "D", "E", "F"]:
            port = getattr(Port, port)
            try:
                self.sensor = ColorDistanceSensor(port)
            except OSError:
                continue
            else:
                break
        self.marker_exit_callback = marker_exit_callback

        self.last_marker_color = None
        self.marker_samples = 0

        self.last_hsv = None
        self.valid_colors = []
        self.initial_hue = 0
        self.initial_chroma = 0

        self.color_buf = bytearray(1004)
        self.buf_index = 0

    def get_marker_color(self):
        h, s, v = self.last_hsv.h, self.last_hsv.s, self.last_hsv.v
        if s * v < io_hub.storage[_CONFIG_CHROMA_THRESHOLD]:
            return None
        colorerr = 181
        found_color = None
        for last_color, chue in enumerate(COLOR_HUES):
            err = abs(((chue - h + 180) % 360) - 180)
            if found_color is None or err < colorerr:
                found_color = last_color
                colorerr = err
        if found_color in self.valid_colors:
            if self.last_marker_color is None:
                self.initial_hue = h
                self.initial_chroma = s * v
            return found_color
        return None

    def update(self, delta):
        self.last_hsv = self.sensor.hsv()

        pack_into(
            ">HBB",
            self.color_buf,
            self.buf_index,
            self.last_hsv.h,
            self.last_hsv.s,
            self.last_hsv.v,
        )
        self.buf_index = (self.buf_index + 4) % 1000

        marker_color = self.get_marker_color()
        if self.last_marker_color is not None:
            if marker_color is None:
                self.marker_exit_callback(self.last_marker_color)
                self.marker_samples = 0
            else:
                self.marker_samples += 1
                if marker_color != self.last_marker_color:
                    pack_into(
                        ">HBB",
                        self.color_buf,
                        self.buf_index,
                        361,
                        1,
                        self.last_marker_color + (marker_color << 4),
                    )
                    self.buf_index = (self.buf_index + 4) % 1000
                    print(
                        "marker color inconsistent:",
                        marker_color,
                        self.last_marker_color,
                    )

        self.last_marker_color = marker_color


class TrainMotor:
    def __init__(self):
        self.speed = 0
        self.target_speed = 0
        self.motors = []
        for port in ["A", "B", "C", "D", "E", "F"]:
            try:
                port = getattr(Port, port)
            except AttributeError:
                break
            try:
                self.motors.append(DCMotor(port))
            except OSError:
                try:
                    self.motors.append(Motor(port))
                except OSError:
                    continue
        self.facing = 1

    def set_facing(self, facing):
        self.facing = facing

    def set_target(self, speed):
        self.target_speed = speed

    def set_speed(self, speed):
        self.target_speed = speed
        self.speed = speed

    def update(self, delta):
        if self.speed * self.facing >= 0:
            if abs(self.speed) < self.target_speed:
                self.speed = (
                    min(
                        abs(self.speed) + delta * io_hub.storage[_CONFIG_MOTOR_ACC],
                        self.target_speed,
                    )
                    * self.facing
                )
            if abs(self.speed) > self.target_speed:
                self.speed = (
                    max(
                        abs(self.speed) - delta * io_hub.storage[_CONFIG_MOTOR_DEC],
                        self.target_speed,
                    )
                    * self.facing
                )
        else:
            self.speed += delta * io_hub.storage[_CONFIG_MOTOR_DEC] * self.facing

        for i, motor in enumerate(self.motors):
            polarity = (io_hub.storage[_CONFIG_MOTOR_INVERTED + i] * -2) + 1
            motor.dc(self.speed * polarity)


class Route:
    def __init__(self):
        self.legs = [
            RouteLeg(
                bytearray(
                    [
                        _SENSOR_SPEED_CRUISE << 6 | _SENSOR_KEY_IN << 4 | _COLOR_ANY,
                        _LEG_FLAG_STOP,
                    ]
                )
            )
        ]
        self.index = 0

    def current_leg(self):
        return self.legs[self.index]

    def next_leg(self):
        try:
            return self.legs[self.index + 1]
        except IndexError:
            return None

    def set_leg(self, data):
        leg_index = data[0]
        if leg_index == len(self.legs):
            self.legs.append(None)
        leg = RouteLeg(data[1:])
        self.legs[leg_index] = leg

    def advance(self):
        self.index += 1
        assert self.index < len(self.legs)
        io_hub.emit_data(bytes((_DATA_LEG_ADVANCE, self.index)))

    def advance_sensor(self, color):
        next_color = self.current_leg().get_next_color()
        if next_color != color and next_color != _COLOR_ANY:
            # print(next_color, color, train.sensor.initial_chroma, train.sensor.initial_hue, train.sensor.marker_samples)
            data = pack(
                ">BBBHHH",
                _DATA_UNEXPECTED_MARKER,
                next_color,
                color,
                train.sensor.initial_chroma,
                train.sensor.initial_hue,
                train.sensor.marker_samples,
            )
            io_hub.emit_data(bytes(data))
            pack_into(
                ">HBB",
                train.sensor.color_buf,
                train.sensor.buf_index,
                361,
                1,
                next_color + (color << 4),
            )
            train.sensor.buf_index = (train.sensor.buf_index + 4) % 1000
            return
        pack_into(">HBB", train.sensor.color_buf, train.sensor.buf_index, 361, 0, color)
        train.sensor.buf_index = (train.sensor.buf_index + 4) % 1000

        current_leg = self.current_leg()
        current_leg.advance_sensor()
        if current_leg.is_complete():
            if not current_leg.intent_stop:
                self.advance()
            elif self.next_leg() is None:
                io_hub.emit_data(bytes((_DATA_ROUTE_COMPLETE, self.index)))

    def get_train_state(self):
        will_turn = False
        current_leg = self.current_leg()

        next_leg = self.next_leg()
        if next_leg is not None:
            will_turn = current_leg.backwards != next_leg.backwards

        return current_leg.get_train_state(will_turn)


class RouteLeg:
    def __init__(self, data):
        self.markers = data[:-1]
        self.intent_stop = bool(data[-1] & _LEG_FLAG_STOP)
        self.backwards = bool(data[-1] & _LEG_FLAG_BACKWARDS)
        self.index = 0
        self.entered = False

    def is_complete(self):
        return self.index == len(self.markers) - 1

    def advance_sensor(self):
        self.index += 1
        assert self.index < len(self.markers)
        io_hub.emit_data(bytes((_DATA_SENSOR_ADVANCE, self.index)))
        if self.get_prev_key() == _SENSOR_KEY_ENTER:
            self.entered = True

    def get_next_color(self):
        return self.markers[self.index + 1] & 0x0F

    def get_prev_speed(self):
        return (self.markers[self.index] >> 6) & 0b11

    def get_prev_key(self):
        return (self.markers[self.index] >> 4) & 0b11

    def get_train_state(self, will_turn):
        speed = self.get_prev_speed()
        if self.intent_stop or will_turn:
            if self.is_complete():
                return _STATE_FLAG_STOP

            if self.entered:
                speed = _SENSOR_SPEED_SLOW

        state = _STATE_FLAG_RUN | speed
        if self.backwards:
            state |= _STATE_FLAG_BACKWARDS
        return state


class Train:
    def __init__(self):
        self.motor = TrainMotor()
        print(self.motor.motors)

        try:
            self.sensor = TrainSensor(self.on_marker_passed)
        except AttributeError:
            self.sensor = None

        print(self.sensor)

        self.route: Route = Route()

    def on_marker_passed(self, color):
        self.route.advance_sensor(color)
        self.set_state(self.route.get_train_state())
        if self.route.next_leg() == None and self.route.current_leg().is_complete():
            self.route = None

    def advance_route(self):
        self.route.advance()
        self.set_state(self.route.get_train_state())

    def set_state(self, state):
        if state & _STATE_FLAG_STOP:
            self.motor.set_speed(0)
            return
        if state & _STATE_FLAG_BACKWARDS:
            self.motor.set_facing(-1)
        else:
            self.motor.set_facing(1)

        if state & _STATE_FLAG_RUN:
            self.motor.set_target(io_hub.storage[2 + (state & 0x0F)])

    def new_route(self):
        self.route = Route()

    def set_route_leg(self, data):
        self.route.set_leg(data)

    def set_leg_intention(self, data):
        self.route.legs[data[0]].intent_stop = bool(data[1])
        if self.route.index == data[0]:
            state = self.route.get_train_state()
            self.set_state(state)

    def update(self, delta):
        if (
            self.sensor is not None
            and self.motor.target_speed != 0
            and len(self.route.legs) > 1
        ):
            self.sensor.update(delta)

        if self.motor is not None:
            self.motor.update(delta)

    def set_valid_colors(self, data):
        self.sensor.valid_colors = list(data)

    def dump_color_buffer(self):
        chroma_threshold = io_hub.storage[_CONFIG_CHROMA_THRESHOLD]
        pack_into(
            ">HH", self.sensor.color_buf, 1000, chroma_threshold, self.sensor.buf_index
        )
        io_hub.dump_data(_DUMP_TYPE_COLORS, self.sensor.color_buf)


assert VERSION != b"1.0.0"
train = Train()
io_hub = IOHub(train)

io_hub.storage[_CONFIG_CHROMA_THRESHOLD] = 3500
io_hub.storage[_CONFIG_MOTOR_ACC] = 40
io_hub.storage[_CONFIG_MOTOR_DEC] = 90
io_hub.storage[_CONFIG_MOTOR_SLOW_SPEED] = 40
io_hub.storage[_CONFIG_MOTOR_CRUISE_SPEED] = 75
io_hub.storage[_CONFIG_MOTOR_FAST_SPEED] = 100
io_hub.storage[_CONFIG_MOTOR_INVERTED] = 0

io_hub.run_loop()
