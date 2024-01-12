from pybricks.hubs import ThisHub
from pybricks.parameters import Color
from pybricks.tools import wait

hub = ThisHub()
hub.light.on(Color.YELLOW)
print("Hello, World!")
while True:
    wait(100)
    hub.light.on(Color.BLUE)
    wait(100)
    hub.light.on(Color.RED)