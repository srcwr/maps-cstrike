import os
import glob
import html
import gzip
import minify_html
import sqlite3

if not os.path.exists("processed"):
    os.makedirs("processed")

conn = sqlite3.connect("processed/maps.db")
cur = conn.cursor()
cur.executescript("""
DROP TABLE IF EXISTS maps_unfiltered;
DROP TABLE IF EXISTS maps_canon;

CREATE TABLE maps_unfiltered (mapname TEXT NOT NULL, filesize INT NOT NULL, filesize_bz2 INT NOT NULL, sha1 TEXT NOT NULL);
CREATE TABLE maps_canon (mapname TEXT NOT NULL, filesize INT NOT NULL, filesize_bz2 INT NOT NULL, sha1 TEXT NOT NULL);

CREATE INDEX mapnameu ON maps_unfiltered(mapname);
CREATE INDEX sha1m on maps_unfiltered(sha1);
CREATE INDEX mapnamec ON maps_canon(mapname);
CREATE INDEX sha1c on maps_canon(sha1);
""")

unique = set()
for filename in glob.glob("unprocessed/*.csv"):
    with open(filename) as f:
        for line in f:
            if line == "mapname,filesize,filesize_bz2,sha1\n":
                continue
            unique.add(line.lower().strip())

unfiltered = set(unique)
for filename in glob.glob("filters/*.csv"):
    with open(filename) as f:
        for line in f:
            unique.remove(line.lower().strip())

cur.executemany("INSERT INTO maps_unfiltered VALUES(?,?,?,?);", [u.split(",") for u in unfiltered])
cur.executemany("INSERT INTO maps_canon VALUES(?,?,?,?);", [u.split(",") for u in unique])

with open("canon.csv") as f:
    things = [line.lower().strip().split(",") for line in f]
    cur.executemany("DELETE FROM maps_canon WHERE mapname = ? AND sha1 != ?;", things)
conn.commit() # fuck you for making me call you

def write_mini(filename, content):
    with open(filename, "w", encoding="utf-8") as h:
        content = minify_html.minify(content, minify_js=True, minify_css=True)
        with gzip.open(filename + ".gz", "wt", encoding="utf-8") as g:
            g.write(content)
        h.write(content)

def create_thing(table, outfilename, canon):
    res = cur.execute(f"SELECT COUNT(*), SUM(s) FROM (SELECT SUM(filesize) s FROM {table} GROUP BY sha1);").fetchone()

    with open("index_top.html", encoding="utf-8") as f:
        index_html = f.read() + """
        <h1>BORN TO DIE</h1>
        <h2>WORLD IS A FUCK</h2>
        <h3>鬼神 Kill Em All {}</h3>
        <h3>I am trash man</h3>
        <h3>{:,} DEAD BYTES</h3>
        <h4>(sorting is slow... you have been warned...)</h4>
        """.format(res[0], res[1])

    index_html += """
    <table id="list" class="sortable">
    <thead>
    <tr>
    <th style="width:1%">Map name</th>
    <th style="width:5%">Hash</th>
    <th style="width:5%">Size bsp</th>
    <th style="width:5%">Size bz2</th>
    </tr>
    </thead>
    <tbody>
    """

    for row in cur.execute(f"SELECT mapname, filesize, filesize_bz2, sha1 FROM {table} ORDER BY mapname;").fetchall():
        if canon:
            index_html += """
            <tr>
            <td><a href="{}.bsp.bz2" download>{}</a></td>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
            </tr>
            """.format(html.escape(row[0]), html.escape(row[0]), row[3], row[1], row[2])
        else:
            index_html += """
            <tr>
            <td><a href="#">{}</a></td>
            <td><a href="{}.bsp.bz2" download>{}</a></td>
            <td>{}</td>
            <td>{}</td>
            </tr>
            """.format(html.escape(row[0]), row[3], row[3], row[1], row[2])

    with open("index_bottom.html", encoding="utf-8") as f:
        write_mini(f"processed/{outfilename}", index_html + f.read())

create_thing("maps_unfiltered", "unfiltered.html", False)
create_thing("maps_canon", "canon.html", True)
