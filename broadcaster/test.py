# Bibliotheken laden
from time import sleep

from machine import Pin

# Initialisierung der Onboard-LED
led_onboard = Pin("LED", Pin.OUT)

# LED einschalten
led_onboard.on()

# 5 Sekunden warten
sleep(5)

# LED ausschalten
led_onboard.off()
