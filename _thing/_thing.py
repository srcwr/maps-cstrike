import os.path
import json
import sqlite3

things = {}

try:
    db = sqlite3.connect("processed/maps.db")
except:
    db = sqlite3.connect("../processed/maps.db")
cur = db.cursor()
for row in cur.execute("SELECT mapname, filesize FROM maps_unfiltered"):
    if not row[0] in things:
        things[row[0]] = []
    things[row[0]].append(int(row[1]))

if os.path.exists("_thing"):
    file = open("_thing/_thing.json", "w")
else:
    file = open("_thing.json", "w")
json.dump(things, file)
