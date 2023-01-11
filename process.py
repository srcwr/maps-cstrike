import os
import glob
import html
import gzip
import minify_html
import sqlite3
import csv

os.makedirs("processed/hashed", exist_ok=True)
os.makedirs("processed/maps", exist_ok=True)

conn = sqlite3.connect("processed/maps.db")
cur = conn.cursor()
cur.executescript("""
DROP TABLE IF EXISTS maps_unfiltered;
DROP TABLE IF EXISTS maps_canon;
DROP TABLE IF EXISTS gamebanana;
DROP TABLE IF EXISTS links;

CREATE TABLE maps_unfiltered (mapname TEXT NOT NULL, filesize INT NOT NULL, filesize_bz2 INT NOT NULL, sha1 TEXT NOT NULL);
CREATE TABLE maps_canon (mapname TEXT NOT NULL, filesize INT NOT NULL, filesize_bz2 INT NOT NULL, sha1 TEXT NOT NULL);
CREATE TABLE gamebanana (sha1 TEXT NOT NULL, gamebananaid INT NOT NULL, gamebananafileid INT NOT NULL);
CREATE TABLE links (sha1 TEXT NOT NULL, url TEXT NOT NULL);

CREATE INDEX mapnameu ON maps_unfiltered(mapname);
CREATE INDEX sha1m on maps_unfiltered(sha1);
CREATE INDEX mapnamec ON maps_canon(mapname);
CREATE INDEX sha1c on maps_canon(sha1);
CREATE INDEX sha1g on gamebanana(sha1);
CREATE INDEX sha1o on links(sha1);
""")

# TODO: remerge maps table & add `canon` column to table...

gamebanana = {}
links = {}

unique = set()
for filename in glob.glob("unprocessed/*.csv"):
    with open(filename, newline='', encoding="utf-8") as f:
        cr = csv.reader(f)
        for line in cr:
            if line[0] == "mapname":
                continue
            thing = [x.lower() for x in line]
            thing[0] = thing[0].strip().replace('.', '_').replace(' ', '_') # because CS:S fails to download maps
            if len(thing) > 4:
                if thing[4].startswith("http://") or thing[4].startswith("https://"):
                    links[thing[3]] = thing[4]
                else:
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
            thing = [x.lower() for x in line][:4]
            thing[0] = thing[0].strip().replace('.', '_').replace(' ', '_').strip() # because CS:S fails to download maps
            unique.remove(tuple(thing))
            #if line == "mapname,filesize,filesize_bz2,sha1\n":
            #    continue
            #unique.remove(line.lower().strip())

cur.executemany("INSERT INTO maps_unfiltered VALUES(?,?,?,?);", unfiltered)
cur.executemany("INSERT INTO maps_canon VALUES(?,?,?,?);", unique)
cur.executemany("INSERT INTO gamebanana VALUES(?,?,?);", [(a,b,c) for a, (b, c) in gamebanana.items()])
cur.executemany("INSERT INTO links VALUES(?,?);", [(a,b) for a, b in links.items()])

with open("canon.csv", encoding="utf-8") as f:
    #things = [[x.lower().strip() for x in line] for line in csv.reader(f)] # also newline='' in open
    things = [line.lower().strip().split(",")[:2] for line in f]
    things.pop(0) # remove "mapname,sha1,note"
    cur.executemany("DELETE FROM maps_canon WHERE mapname = ? AND sha1 != ?;", things)
conn.commit() # fuck you for making me call you

def create_thing(table, outfilename, canon, title):
    res = cur.execute(f"SELECT COUNT(*), SUM(s1), SUM(s2) FROM (SELECT SUM(filesize) s1, SUM(filesize_bz2) s2 FROM {table} GROUP BY sha1);").fetchone()

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
        <h3>BZ2 size: {:,} BYTES</h3>
        <h4>(sorting is slow... you have been warned...)</h4>
        """.format(title, res[0], res[1], res[2])

    outf = open(f"processed/{outfilename}", "w+", encoding="utf-8")
  
    outf.write(index_html + """
    <table id="list" class="sortable">
    <thead>
    <tr>
    <th style="width:1%">Map name</th>
    <th style="width:5%">Hash</th>
    <th style="width:5%">Size bsp</th>
    <th style="width:5%">Size bz2</th>
    <th style="width:5%">Page</th>
    </tr>
    </thead>
    <tbody>
    """)

    groupy = ""
    if canon:
        groupy = "GROUP BY mapname"
    for row in cur.execute(f"SELECT mapname, filesize, filesize_bz2, m.sha1, gamebananaid, url FROM {table} m LEFT JOIN gamebanana g ON g.sha1 = m.sha1 LEFT JOIN links l ON l.sha1 = m.sha1 {groupy} ORDER BY mapname;").fetchall():
        link = row[5]
        if link != None:
            link = f'<td><a href="{link}">clickme</a></td>'
        else:
            gbid = row[4]
            if gbid == None:
                link = ""
            else:
                link = f'<td><a href="https://gamebanana.com/mods/{gbid}">{gbid}</a></td>'
        if canon:
            index_html = """
            <tr>
            <td><a href="#">{}</a></td>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
            {}
            </tr>
            """.format(html.escape(row[0]), row[3], row[1], row[2], link)
        else:
            #<td><a href="#">{}</a></td>
            index_html = """
            <tr>
            <td><a href="#">{}</a></td>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
            {}
            </tr>
            """.format(html.escape(row[0]), row[3], row[1], row[2], link)
        outf.write(index_html)

    outf.seek(0)
    content = minify_html.minify(outf.read() + open("index_bottom.html", encoding="utf-8").read(), minify_js=True, minify_css=True)
    outf.seek(0)
    outf.truncate(0)
    outf.write(content)
    with gzip.open(f"processed/{outfilename}.gz", "wt", encoding="utf-8") as g:
        g.write(content)

create_thing("maps_unfiltered", "hashed/index.html", False, "hashed/unfiltered maps")
create_thing("maps_canon", "maps/index.html", True, "canon/filtered maps")
