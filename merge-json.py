#!/usr/bin/python3

import argparse
import collections
import datetime
import json
import typing
import sys


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


def merge(existing_data: dict, new_data: dict) -> dict:
    """Keep only results that appear in both sets."""
    results = {}
    for category in existing_data:
        if category not in new_data:
            continue
        distance = existing_data[category]["distance"] + new_data[category]["distance"]
        climb = existing_data[category]["climb"] + new_data[category]["climb"]
        results[category] = {
            "name": category,
            "distance": distance,
            "climb": climb,
            "results": [],
        }

        merged_results = []
        for result in existing_data[category]["results"]:
            if not is_valid_result(result):
                continue

            for other_result in new_data[category]["results"]:
                if not is_valid_result(other_result):
                    continue
                if result["name"] == other_result["name"]:
                    total_time = parse_time(result["time"]) + parse_time(other_result["time"])
                    merged_results.append(
                        {
                            "position": 1,
                            "name": result["name"],
                            "club": result["club"],
                            "time": total_time,
                            "status": "OK",
                            "ageclass": result["ageclass"],
                        }
                    )
                    break

        final_results = []
        for position, result in enumerate(
            sorted(merged_results, key=lambda r: r["time"])
        ):
            final_results.append(
                {**result, "position": position + 1, "time": format_time(result["time"])}
            )

        results[category]["results"] = final_results

    return results


def run(paths: typing.IO, date: str, name: str, location: str):
    """Calculate the combined times of multiple Helga webres JSON."""
    data = None
    for open_path in paths:
        if data is None:
            data = json.load(open_path)["categories"]
        else:
            data = merge(data, json.load(open_path)["categories"])

    json.dump(
        dict(
            date=date,
            name=name,
            location=location,
            categories=data,
        ),
        sys.stdout,
        indent=4,
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--date", type=str, required=True)
    parser.add_argument("--name", type=str, required=True)
    parser.add_argument("--location", type=str, required=True)
    parser.add_argument("open_paths", metavar="paths", type=open, nargs="+")
    args = parser.parse_args()
    run(args.open_paths, args.date, args.name, args.location)
