# Bibliotheken laden
import sys
from time import sleep

from machine import Pin

led_onboard = Pin("LED", Pin.OUT)

while True:
    print("Waiting for input (type 'on' or 'off')...")
    line = sys.stdin.readline().strip()
    print(f"Received input: '{line}'")
    if not line:
        continue
    if "on" in line:
        led_onboard.on()
    elif "off" in line:
        led_onboard.off()
    else:
        print("Invalid input. Please type 'on' or 'off'.")
