#!/bin/sh
# Apply the keyboard's udev rule now, so the app works without replugging the keyboard.
udevadm control --reload || true
udevadm trigger --subsystem-match=hidraw || true
