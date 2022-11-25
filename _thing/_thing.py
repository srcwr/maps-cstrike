import os.path
import json
import csv

things = {}
f = None
try:
    f = open("processed/maps.csv", "r", newline="")
except:
    f = open("../processed/maps.csv", "r", newline="")

reader = csv.reader(f)

for row in reader:
    name = row[0]
    if not name in things:
        things[name] = []
    things[name].append(int(row[1]))

f = None
if os.path.exists("_thing"):
    file = open("_thing/_thing.json", "w")
else:
    file = open("_thing.json", "w")
json.dump(things, file)
