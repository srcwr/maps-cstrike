# SPDX-License-Identifier: WTFPL
# Copyright 2022-2025 rtldg <rtldg@protonmail.com>

# /// script
# requires-python = ">=3.10,<3.13"
# dependencies = [
#   "minify-html==0.15.0",
# ]
# ///

import os
import glob
import html
import gzip
import minify_html
import sqlite3
import csv
import shutil
import json
import subprocess
import sys

"""
os.makedirs("processed/hashed", exist_ok=True)
os.makedirs("processed/maps", exist_ok=True)
"""

conn = sqlite3.connect(":memory:")
cur = conn.cursor()
cur.executescript("""
DROP TABLE IF EXISTS maps_unfiltered;
DROP TABLE IF EXISTS maps_canon;
DROP TABLE IF EXISTS maps_czarchasm;
DROP TABLE IF EXISTS maps_ksfthings;
DROP TABLE IF EXISTS gamebanana;
DROP TABLE IF EXISTS links;

CREATE TABLE maps_unfiltered (mapname TEXT NOT NULL, filesize INT NOT NULL, filesize_bz2 INT NOT NULL, sha1 TEXT NOT NULL);
CREATE TABLE maps_canon (mapname TEXT NOT NULL, filesize INT NOT NULL, filesize_bz2 INT NOT NULL, sha1 TEXT NOT NULL);
CREATE TABLE maps_czarchasm (mapname TEXT NOT NULL, filesize INT NOT NULL, filesize_bz2 INT NOT NULL, sha1 TEXT NOT NULL);
CREATE TABLE maps_ksfthings (mapname TEXT NOT NULL, filesize INT NOT NULL, filesize_bz2 INT NOT NULL, sha1 TEXT NOT NULL);
CREATE TABLE gamebanana (sha1 TEXT NOT NULL, gamebananaid INT NOT NULL, gamebananafileid INT NOT NULL);
CREATE TABLE links (sha1 TEXT NOT NULL, url TEXT NOT NULL);

CREATE INDEX mapnameu ON maps_unfiltered(mapname);
CREATE INDEX sha1m on maps_unfiltered(sha1);
CREATE INDEX mapnamec ON maps_canon(mapname);
CREATE INDEX sha1c on maps_canon(sha1);
CREATE INDEX mapnamecz ON maps_czarchasm(mapname);
CREATE INDEX sha1cz on maps_czarchasm(sha1);
CREATE INDEX mapnameksf ON maps_ksfthings(mapname);
CREATE INDEX sha1ksf on maps_ksfthings(sha1);
CREATE INDEX sha1g on gamebanana(sha1);
CREATE INDEX sha1o on links(sha1);
""")

# TODO: remerge maps table & add `canon` column to table...

def normal_name(m):
    return m.strip().replace('.', '_').lower()


def glob_unprocessed_csvs(pattern):
    #i = 0
    gamebanana = {}
    links = {}
    unique = set()
    for filename in glob.glob(pattern):
        with open(filename, newline='', encoding="utf-8") as f:
            cr = csv.reader(f)
            for line in cr:
                if line[0] == "mapname" or line[0][0] == "#":
                    continue
                thing = [x.lower() for x in line]
                thing[0] = normal_name(thing[0]) # because CS:S fails to download maps with '.'
                if len(thing) > 4:
                    if thing[4].startswith("http://") or thing[4].startswith("https://"):
                        links[thing[3]] = thing[4]
                    else:
                        # path & maybe gamebanana path...
                        splits = thing[4].split('_')
                        if splits[0].isdigit() and splits[0] != "0" and splits[1].isdigit(): # might have false positives...
                            gamebanana[thing[3]] = (int(splits[0]), int(splits[1]))
                #i += 1
                unique.add(tuple(thing[:4]))
    #print(i, len(unique))
    return (gamebanana, links, unique)

(gamebanana, links, unique) = glob_unprocessed_csvs("unprocessed/*.csv")
(_, _, czarchasm_unique) = glob_unprocessed_csvs("unprocessed/hashed_bsps_czar_p*.csv")
(_, _, ksfthings) = glob_unprocessed_csvs("unprocessed/ksf - github.com OuiSURF Surf_Maps.csv")

def glob_filters(pattern, mapset):
    for filename in glob.glob(pattern):
        with open(filename, newline='', encoding="utf-8") as f:
            cr = csv.reader(f)
            for line in cr:
                if line[0] == "mapname" or line[0].startswith("#"):
                    continue
                thing = [x.lower() for x in line][:4]
                thing[0] = normal_name(thing[0]) # because CS:S fails to download maps with '.'
                mapset.remove(tuple(thing))
                #if line == "mapname,filesize,filesize_bz2,sha1\n":
                #    continue
                #unique.remove(line.lower().strip())

unfiltered = set(unique)
glob_filters("filters/*.csv", unique)
glob_filters("filters/custom/czarchasm_filter.csv", czarchasm_unique)

cur.executemany("INSERT INTO maps_unfiltered VALUES(?,?,?,?);", unfiltered)
cur.executemany("INSERT INTO maps_canon VALUES(?,?,?,?);", unique)
cur.executemany("INSERT INTO maps_czarchasm VALUES(?,?,?,?);", czarchasm_unique)
cur.executemany("INSERT INTO maps_ksfthings VALUES(?,?,?,?);", ksfthings)
cur.executemany("INSERT INTO gamebanana VALUES(?,?,?);", [(a,b,c) for a, (b, c) in gamebanana.items()])
cur.executemany("INSERT INTO links VALUES(?,?);", [(a,b) for a, b in links.items()])

with open("canon.csv", encoding="utf-8") as f:
    #things = [[x.lower().strip() for x in line] for line in csv.reader(f)] # also newline='' in open
    things = [line.lower().strip().split(",")[:2] for line in f if line[0] != "#"]
    things.pop(0) # remove "mapname,sha1,note"
    for x in things:
        if len(x[1]) != 40:
            raise Exception(f"fuck you {x} -- check the line to make sure it's map,hash,note instead of map,size,something,hash,note")
    cur.executemany("DELETE FROM maps_canon WHERE mapname = ? AND sha1 != ?;", things)
conn.commit() # fuck you for making me call you
try:
    os.remove("processed/maps.db")
except:
    pass
cur.execute("VACUUM INTO 'processed/maps.db'")

recently_added = []
with open("recently_added.csv", newline='', encoding="utf-8") as f:
    cr = csv.reader(f)
    for line in cr:
        if line[0][0] == "#":
            continue
        line[0] = normal_name(line[0])
        splits = line[4].split('_')
        if splits[0].isdigit() and splits[0] != "0" and splits[1].isdigit(): # might have false positives...
            if len(line[5]) > 0:
                line[5] = '<a href="https://gamebanana.com/mods/{}">gamebanana</a> - '.format(splits[0]) + line[5]
            else:
                line[5] = '<a href="https://gamebanana.com/mods/{}">gamebanana</a>'.format(splits[0])
        recently_added.append(line)
        if len(recently_added) > 155:
            break
    recently_added.pop(0) # remove "mapname,filesize,filesize_bz2,sha1,note,recently_added_note,datetime"

"""
def create_json(table, outfilename):
    map = {}
    for pair in cur.execute(f"SELECT mapname, sha1 FROM {table}"):
        map[pair[0]] = pair[1]
    outtextffff = open(f"processed/{outfilename}.txt", "w", newline="\n", encoding="utf-8")
    outtextffff.write(json.dumps(map, separators=(',', ':')))

def create_binary(table, outfilename):
    map = {}
    for pair in cur.execute(f"SELECT mapname, sha1 FROM {table}"):
        map[pair[0]] = pair[1]
    map = dict(sorted(map.items()))
    outtextffff = open(f"processed/{outfilename}.txt", "wb")
    for pair in map:
        outtextffff.write(b'\0')
        outtextffff.write(pair[0].encode())
        outtextffff.write(b'\0')
        outtextffff.write(bytes.fromhex(pair[1]))
"""

def create_thing(table, outfilename, canon, title, sqlwhere, omit_recently_added, txt_as_urls):
    res = cur.execute(f"SELECT COUNT(*), SUM(s1), SUM(s2) FROM (SELECT SUM(filesize) s1, SUM(filesize_bz2) s2 FROM {table} {sqlwhere} GROUP BY sha1);").fetchone()

    hcpath = outfilename.rsplit(".", maxsplit=1)[0].rsplit("/", maxsplit=1)[1]

    with open("index_top.html", encoding="utf-8") as f:
        index_html = """
        <!DOCTYPE html>
        <html>
        <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <meta name="viewport" content="width=device-width">
        <title>fastdl.me {}</title>
        """.format(title.replace("<br>", " ")) + f.read() + """
        <h1>fastdl.me {}</h1>
        page hit count: <img height=14 width=92 alt=hc src="https://hc.fastdl.me/hc/{}.jpg"><br>
        <h2><a href="https://fastdl.me">homepage</a></h2>
        <h3>Number of maps: {}</h3>
        <h3>Unpacked size: {:,} BYTES</h3>
        <h3>BZ2 size: {:,} BYTES</h3>
        links to other versions of this list: <a href="https://{}.txt">txt</a> / <a href="https://{}.csv">csv</a>
        <br>&nbsp;
        """.format(title, hcpath, res[0], res[1], res[2], outfilename, outfilename)

    if not omit_recently_added:
        index_html += """
        <br>
        <h2>Recently added:</h2>
        <a href="https://github.com/srcwr/maps-cstrike/commits/master">(full commit history)</a>
        <table id="recentlyadded">
        <thead>
        <tr>
        <th style="width:1%">Map name</th>
        <th style="width:1%">SHA-1 Hash</th>
        <th style="width:15%">Note</th>
        <th style="width:2%">Date added</th>
        </tr>
        </thead>
        <tbody>
        """
        #<th style="width:1%">List of packed files</th>
        for x in recently_added:
            index_html += """
            <tr>
            <td><a href="#">{}</a></td>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
            </tr>
            """.format(html.escape(x[0]), x[3], x[5], x[6], x[3])
            #<td><a href="https://github.com/srcwr/maps-cstrike-more/blob/master/filelist/{}.csv">{}</a></td>

        index_html += '</tbody></table>'

    outf = open(f"processed/{outfilename}", "w+", encoding="utf-8")

    outf.write(index_html + """
    <h4>(sorting is slow... you have been warned...)</h4>
    <table id="list" class="sortable">
    <thead>
    <tr>
    <th style="width:1%">Map name</th>
    <th style="width:5%">SHA-1 Hash</th>
    <th style="width:5%">Size bsp</th>
    <th style="width:5%">Size bz2</th>
    <th style="width:5%">Page</th>
    </tr>
    </thead>
    <tbody>
    """)

    outtextffff = open(f"processed/{outfilename}.txt", "w", newline="\n", encoding="utf-8")
    hashies = set()

    outcsvffff = open(f"processed/{outfilename}.csv", "w+", newline="", encoding="utf-8")
    mycsv = csv.writer(outcsvffff)
    mycsv.writerow(["mapname","sha1","filesize","filesize_bz2","url"])

    groupy = ""
    fzy = "filesize_bz2"
    if canon:
        groupy = "GROUP BY mapname"
        fzy = "MAX(filesize_bz2)"
    for row in cur.execute(f"""
        SELECT mapname, filesize, {fzy}, m.sha1, gamebananaid, url
        FROM {table} m
        LEFT JOIN gamebanana g ON g.sha1 = m.sha1
        LEFT JOIN links l ON l.sha1 = m.sha1
        {sqlwhere}
        {groupy}
        ORDER BY mapname;""").fetchall():
        link = row[5]
        htmllink = ""
        if link != None:
            htmllink = f'<td><a href="{link}">clickme</a></td>'
        else:
            gbid = row[4]
            if gbid != None and gbid != 0:
                link = "https://gamebanana.com/mods/" + str(gbid)
                htmllink = f'<td><a href="{link}">{gbid}</a></td>'
        if txt_as_urls:
            outtextffff.write(f"http://main.fastdl.me/hashed/{row[3]}/{row[0]}.bsp.bz2\n")
        #if "canon" in table:
        elif canon:
            outtextffff.write(row[0] + "\n")
        else:
            hashies.add(row[3])

        mycsv.writerow([row[0], row[3], row[1], row[2], link])
        if canon:
            index_html = """
            <tr>
            <td><a href="#">{}</a></td>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
            {}
            </tr>
            """.format(html.escape(row[0]), row[3], row[1], row[2], htmllink)
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
            """.format(html.escape(row[0]), row[3], row[1], row[2], htmllink)
        outf.write(index_html)

    if table == "maps_unfiltered":
        hashies = sorted(hashies)
        for hash in hashies:
            outtextffff.write(hash + "\n")

    outf.seek(0)
    content = minify_html.minify(outf.read() + open("index_bottom.html", encoding="utf-8").read(), minify_js=True, minify_css=True)
    outf.seek(0)
    outf.truncate(0)
    outf.write(content)
    #"""
    with gzip.open(f"processed/{outfilename}.gz", "wt", encoding="utf-8") as g:
        g.write(content)
    #"""

try:
    shutil.rmtree("processed/check.fastdl.me")
except:
    pass
shutil.copytree("fastdlsite/check.fastdl.me", "processed/check.fastdl.me")
_things = {}
for row in cur.execute("SELECT mapname, filesize FROM maps_unfiltered"):
    if not row[0] in _things:
        _things[row[0]] = []
    _things[row[0]].append(int(row[1]))
with open("processed/check.fastdl.me/_thing.json", "w") as f:
    json.dump(_things, f)

try:
    shutil.rmtree("processed/fastdl.me")
except:
    pass
shutil.copytree("fastdlsite/fastdl.me", "processed/fastdl.me")
with open("processed/fastdl.me/index.html", "wb") as f:
    f.write(open("fastdlsite/fastdl.me/index.html", "rb").read().replace(b'<!-- embed the privacy policy here -->', open("fastdlsite/embedded-privacy-policy.html", "rb").read()))

try:
    shutil.rmtree("processed/main.fastdl.me")
except:
    pass
shutil.copytree("fastdlsite/main.fastdl.me", "processed/main.fastdl.me")
shutil.copytree("../fastdl_opendir/materials", "processed/main.fastdl.me/materials")
shutil.copytree("../fastdl_opendir/sound", "processed/main.fastdl.me/sound")
shutil.copy("LICENSE", "processed/main.fastdl.me/WTFPL.txt")
shutil.copy("LICENSE", "processed/fastdl.me/WTFPL.txt")

# On Cloudflare: I have /maps/ rewritten to maps_index.html & /hashed/ rewritten to hashed_index.html....
create_thing("maps_unfiltered", "main.fastdl.me/hashed_index.html", False, "hashed/unfiltered maps", "", False, False)
create_thing("maps_canon", "main.fastdl.me/maps_index.html", True, "canon/filtered maps", "", False, False)
create_thing("maps_canon", "main.fastdl.me/69.html", True, "movement maps (mostly)", "WHERE mapname LIKE 'bh%' OR mapname LIKE 'xc\\_%' ESCAPE '\\' OR mapname LIKE 'kz%' OR mapname LIKE 'surf%' OR mapname LIKE 'tsurf%' OR mapname LIKE 'trikz%' OR mapname LIKE 'jump%' OR mapname LIKE 'climb%' OR mapname LIKE 'fu\\_%' ESCAPE '\\' OR mapname LIKE '%hop%'", False, False)
create_thing("maps_czarchasm", "main.fastdl.me/maps_czarchasm.html", True, 'mirror of maps from <a href="https://czarchasm.club/">czarchasm.club</a>', "", True, True)
create_thing("maps_ksfthings", "main.fastdl.me/maps_ksfthings.html", False, 'mirror of ksf maps<br>from <a href="https://github.com/OuiSURF/Surf_Maps">https://github.com/OuiSURF/Surf_Maps</a><br>(up till 2025-04-06)', "", True, True)

# TODO: generate main.fastdl.me/index.html open directory pages

#subprocess.run('sqlite3 processed/maps.db ".dump maps_canon"', stdout=open("processed/maps.sql", "w"))
#subprocess.run("wrangler d1 execute fastdldb --file=processed/maps.sql", shell=True)

# me-check
if not os.path.isfile("../secretwebhook") or (len(sys.argv) > 1 and sys.argv[1] == "0"):
    sys.exit(0)

# what the fuck wrangler why won't my functions work otherwise
cwd = os.getcwd()
os.chdir("processed/check.fastdl.me")
subprocess.run("npx --yes wrangler pages deploy --commit-dirty=true --project-name check-fastdl --branch main   .", shell=True)
os.chdir(cwd + "/processed/fastdl.me")
subprocess.run("npx --yes wrangler pages deploy --commit-dirty=true --project-name fdl          --branch master .", shell=True)
os.chdir(cwd)

#wrangler pages publish --project-name fdl --branch master processed/fastdl.me
#wrangler pages publish --project-name mfdl --branch master processed/main.fastdl.me

#create_json("maps_canon", "canon.json")
#create_binary("maps_canon", "canon.bin")
