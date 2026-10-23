#!/usr/bin/env python3
"""Resolve simctl's available-device JSON on stdin to one concrete iOS UDID.

An exact name or UDID can be requested; supplying both requires both to match.
Without either override only iPhones qualify. Prefer the newest numeric iOS
runtime, then lexicographically smallest name and UDID, independent of JSON
ordering. This also defines the policy for duplicate names. Only inspect the
inventory: never create, boot, erase, or otherwise mutate a simulator.
"""

import argparse
import json
import re
import sys


IOS_RUNTIME = re.compile(r"com\.apple\.CoreSimulator\.SimRuntime\.iOS-(\d+(?:-\d+){0,2})")
UDID = re.compile(r"[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}")


def select_simulator(inventory, name="", udid=""):
    if udid and not UDID.fullmatch(udid):
        raise ValueError("SIM_UDID must be a simulator UUID")
    if not isinstance(inventory, dict) or not isinstance(inventory.get("devices"), dict):
        raise ValueError("simctl JSON must contain a devices object")

    candidates = []
    seen = set()
    for runtime, devices in inventory["devices"].items():
        match = IOS_RUNTIME.fullmatch(runtime)
        if not match:
            continue  # tvOS/watchOS/visionOS are not iOS test destinations.
        if not isinstance(devices, list):
            raise ValueError("simctl iOS devices must be an array")
        version = tuple(int(part) for part in match[1].split("-"))
        version += (0,) * (3 - len(version))
        for device in devices:
            if not isinstance(device, dict) or type(device.get("isAvailable")) is not bool:
                raise ValueError("simctl iOS device must declare boolean isAvailable")
            if not device["isAvailable"]:
                continue
            device_name, device_id = device.get("name"), device.get("udid")
            if not isinstance(device_name, str) or not device_name.strip():
                raise ValueError("available simctl iOS device must have a name")
            if not isinstance(device_id, str) or not UDID.fullmatch(device_id):
                raise ValueError("available simctl iOS device must have a valid UDID")
            device_id = device_id.upper()
            if device_id in seen:
                raise ValueError("simctl JSON contains a duplicate available UDID")
            seen.add(device_id)
            if name and device_name != name:
                continue
            if udid and device_id != udid.upper():
                continue
            if not name and not udid and not device_name.startswith("iPhone "):
                continue
            candidates.append((tuple(-part for part in version), device_name, device_id))

    if not candidates:
        if name or udid:
            raise ValueError("No available iOS simulator matches SIM_NAME/SIM_UDID")
        raise ValueError("No available iPhone simulator found for the simulator test run")
    return min(candidates)[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--name", default="")
    parser.add_argument("--udid", default="")
    args = parser.parse_args()
    try:
        selected = select_simulator(json.load(sys.stdin), args.name, args.udid)
    except ValueError as error:
        print("Cannot select simulator: " + str(error), file=sys.stderr)
        return 1
    print(selected)
    return 0


if __name__ == "__main__":
    sys.exit(main())
