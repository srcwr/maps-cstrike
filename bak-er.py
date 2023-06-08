# SPDX-License-Identifier: WTFPL

# script I needed when extracting archives with `.bak` files when I was trying to get some timestamps

import csv
import glob
import shutil
from pathlib import Path

with open("unprocessed/gamebanana-everything-rofl-bsp.bak.csv", newline='', encoding="utf-8") as f:
    cr = csv.reader(f)
    for line in cr:
        if line[0] == "mapname" or line[0][0] == "#":
            continue
        archive = Path(line[4]).parts[0]
        for filename in glob.iglob("../gamebanana-scrape/{}*".format(archive)):
            print(filename)
            shutil.copy2(filename, "../temp")
