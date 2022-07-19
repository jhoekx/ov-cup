
import fileinput

import requests

def run():
    for line in fileinput.input():
        if line.strip() == "":
            continue
        category = line.strip()
        response = requests.get(f"https://rankings.orienteering.vlaanderen/cgi-bin/cup-cgi?cup=forest-cup&season=2022&ageClass={category}")
        response.raise_for_status()
        data = response.json()

        print(category)
        count = 1
        for result in data:
            print(f"{count}.,{result['name']},{result['club']},{result['totalScore']}")
            count = count + 1
            if count == 4:
                break
        print("")

if __name__ == "__main__":
    run()
