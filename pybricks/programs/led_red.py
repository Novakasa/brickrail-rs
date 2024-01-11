from pybricks.hubs import CityHub
from pybricks.parameters import Color
from pybricks.tools import wait

hub = CityHub()
hub.light.on(Color.RED)
while True:
    wait(1000)