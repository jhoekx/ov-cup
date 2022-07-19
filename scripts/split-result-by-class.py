#!/usr/bin/python3
"""Merge a webres JSON output by class into a result by course."""

import datetime
import json
import sys

from collections import defaultdict


def parse_time(time: str) -> float:
    parts = time.split(".")
    ms = 0
    if len(parts) == 2:
        ms = float("0." + parts[1])
    hh, mm, ss = parts[0].split(":")
    return int(hh) * 3600 + int(mm) * 60 + int(ss) + ms


def format_time(time: float) -> str:
    return (
        (datetime.datetime.min + datetime.timedelta(seconds=time))
        .time()
        .isoformat(timespec="milliseconds")
    )


def is_valid_result(result: dict) -> bool:
    """Return true when the status is OK and position is not 0."""
    return result["position"] > 0 and result["status"] == "OK"


def _by_distance(category: tuple[str, int, int]) -> int:
    return category[1]

def _by_time(result: dict) -> float:
    return parse_time(result["time"])


def merge(data: dict):
    grouped_results = defaultdict(list)
    for category in data["categories"].values():
        grouped_results[(category["name"][0], category["distance"], category["climb"])].extend(category["results"])

    event = {k: v for k, v in data.items() if k != "categories"}
    event["categories"] = {}
    for gender in ["D", "H"]:
        for index, key in enumerate(key for key in sorted(grouped_results.keys(), key=_by_distance, reverse=True) if key[0] == gender):
            _, distance, climb = key
            course = f"{gender}:{index+1:02d}"
            results = [result for result in grouped_results[key] if is_valid_result(result)]
            results = sorted(results, key=_by_time)
            for position, result in enumerate(results, 1):
                result["position"] = position
            event["categories"][course] = {
                "name": course,
                "distance": distance,
                "climb": climb,
                "results": results,
            }

    json.dump(event, sys.stdout, indent=2)


def run():
    data_path = sys.argv[1]
    data = json.load(open(data_path))
    merge(data)


if __name__ == "__main__":
    run()
