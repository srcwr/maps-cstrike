import os
import glob
import html
import gzip
import minify_html
import sqlite3
import csv

if not os.path.exists("processed"):
    os.makedirs("processed")

conn = sqlite3.connect("processed/maps.db")
cur = conn.cursor()
cur.executescript("""
DROP TABLE IF EXISTS maps_unfiltered;
DROP TABLE IF EXISTS maps_canon;
DROP TABLE IF EXISTS gamebanana;

CREATE TABLE maps_unfiltered (mapname TEXT NOT NULL, filesize INT NOT NULL, filesize_bz2 INT NOT NULL, sha1 TEXT NOT NULL);
CREATE TABLE maps_canon (mapname TEXT NOT NULL, filesize INT NOT NULL, filesize_bz2 INT NOT NULL, sha1 TEXT NOT NULL);
CREATE TABLE gamebanana (sha1 TEXT NOT NULL, gamebananaid INT NOT NULL, gamebananafileid INT NOT NULL);

CREATE INDEX mapnameu ON maps_unfiltered(mapname);
CREATE INDEX sha1m on maps_unfiltered(sha1);
CREATE INDEX mapnamec ON maps_canon(mapname);
CREATE INDEX sha1c on maps_canon(sha1);
CREATE INDEX sha1g on gamebanana(sha1);
""")

gamebanana = {}

unique = set()
for filename in glob.glob("unprocessed/*.csv"):
    with open(filename, newline='', encoding="utf-8") as f:
        cr = csv.reader(f)
        for line in cr:
            if line[0] == "mapname":
                continue
            thing = [x.lower() for x in line]
            if len(thing) > 4:
                # path & maybe gamebanana path...
                splits = thing[4].split('_')
                if splits[0].isdigit() and splits[1].isdigit(): # might have false positives...
                    gamebanana[thing[3]] = (int(splits[0]), int(splits[1]))
            unique.add(tuple(thing[:4]))

unfiltered = set(unique)
for filename in glob.glob("filters/*.csv"):
    with open(filename, newline='', encoding="utf-8") as f:
        cr = csv.reader(f)
        for line in cr:
            if line[0] == "mapname":
                continue
            unique.remove(tuple([x.lower() for x in line]))
            #if line == "mapname,filesize,filesize_bz2,sha1\n":
            #    continue
            #unique.remove(line.lower().strip())

cur.executemany("INSERT INTO maps_unfiltered VALUES(?,?,?,?);", unfiltered)
cur.executemany("INSERT INTO maps_canon VALUES(?,?,?,?);", unique)
cur.executemany("INSERT INTO gamebanana VALUES(?,?,?);", [(a,b,c) for a, (b, c) in gamebanana.items()])

with open("canon.csv", encoding="utf-8") as f:
    #things = [[x.lower().strip() for x in line] for line in csv.reader(f)] # also newline='' in open
    things = [line.lower().strip().split(",")[:2] for line in f]
    things.pop(0) # remove "mapname,sha1,note"
    cur.executemany("DELETE FROM maps_canon WHERE mapname = ? AND sha1 != ?;", things)
conn.commit() # fuck you for making me call you

def write_mini(filename, content):
    with open(filename, "w", encoding="utf-8") as h:
        content = minify_html.minify(content, minify_js=True, minify_css=True)
        with gzip.open(filename + ".gz", "wt", encoding="utf-8") as g:
            g.write(content)
        h.write(content)

def create_thing(table, outfilename, canon, title):
    res = cur.execute(f"SELECT COUNT(*), SUM(s) FROM (SELECT SUM(filesize) s FROM {table} GROUP BY sha1);").fetchone()

    with open("index_top.html", encoding="utf-8") as f:
        index_html = """
        <!DOCTYPE html>
        <html>
        <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <meta name="viewport" content="width=device-width">
        <title>fastdl.me {}</title>
        """.format(title) + f.read() + """
        <h1>fastdl.me {}</h1>
        <h2><a href="https://fastdl.me">homepage</a></h2>
        <h3>Number of maps: {}</h3>
        <h3>Unpacked size: {:,} BYTES</h3>
        <h4>(sorting is slow... you have been warned...)</h4>
        """.format(title, res[0], res[1])

    index_html += """
    <table id="list" class="sortable">
    <thead>
    <tr>
    <th style="width:1%">Map name</th>
    <th style="width:5%">Hash</th>
    <th style="width:5%">Size bsp</th>
    <th style="width:5%">Size bz2</th>
    <th style="width:5%">Gamebanana page</th>
    </tr>
    </thead>
    <tbody>
    """

    for row in cur.execute(f"SELECT mapname, filesize, filesize_bz2, m.sha1, gamebananaid FROM {table} m LEFT JOIN gamebanana g ON g.sha1 = m.sha1 ORDER BY mapname;").fetchall():
        gbid = row[4]
        if gbid == None:
            gbid = ""
        else:
            gbid = f'<td><a href="https://gamebanana.com/mods/{gbid}">{gbid}</a></td>'
        if canon:
            index_html += """
            <tr>
            <td><a href="{}.bsp.bz2" download>{}</a></td>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
            {}
            </tr>
            """.format(html.escape(row[0]), html.escape(row[0]), row[3], row[1], row[2], gbid)
        else:
            #<td><a href="#">{}</a></td>
            index_html += """
            <tr>
            <td>{}</td>
            <td><a href="{}.bsp.bz2" download>{}</a></td>
            <td>{}</td>
            <td>{}</td>
            {}
            </tr>
            """.format(html.escape(row[0]), row[3], row[3], row[1], row[2], gbid)

    with open("index_bottom.html", encoding="utf-8") as f:
        write_mini(f"processed/{outfilename}", index_html + f.read())

create_thing("maps_unfiltered", "hashed/index.html", False, "hashed/unfiltered maps")
create_thing("maps_canon", "maps/index.html", True, "canon/filtered maps")
