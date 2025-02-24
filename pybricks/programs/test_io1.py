from io_hub_unfrozen import IOHub

class TestDevice:
    
    def update(self, delta):
        pass

    def respond(self, data):
        io_hub.emit_data(data)
    
    def print_data(self, data):
        print("printing the data:", repr(data))

    def dump_buffer(self, args):
        buf = bytearray(1000)
        for i in range(1000):
            buf[i] = i%256
        io_hub.dump_data(0, buf)

device = TestDevice()
io_hub = IOHub(device)

print("test")

io_hub.emit_data(b"hello world!")

print("post test")

io_hub.last_output = None
io_hub.emit_data(bytes([4, 2, 0, 69]))

print("post test 2")