# SPDX-License-Identifier: WTFPL

from flask import Flask,redirect,g,request
import sqlite3
from pathlib import Path

HASHFOLDER = "/hashed"

app = Flask(__name__)

def get_db():
    db = getattr(g, '_database', None)
    if db is None:
        db = g._database = sqlite3.connect("file:/www/maps.db?mode=ro", uri=True)
    return db

@app.teardown_appcontext
def close_connection(exception):
    db = getattr(g, '_database', None)
    if db is not None:
        db.close()


def logmap(hash, mapname, folder):
    referer = ""
    if folder == "maps" and request.referrer.startswith("hl2://"):
        referer = request.referer
    things = ["time", hash, mapname, folder]
    pass
def log404(mapname):
    things = ["time", mapname]
    pass
#def log404bsp(mapname):
#    pass


def smells_like_sha1(s):
    try:
        x = int(s, 16)
        return len(s) == 40
    except:
        return False

"""
@app.route('/mapsredir/<mapname>.bsp.bz2')
def mapsredir_mapname(mapname):
    cur = get_db().cursor()
    cur.execute("SELECT sha1, MAX(filesize_bz2) fbz2 FROM maps_canon WHERE mapname = ?", (mapname.lower(),))
    res = cur.fetchone()
    if res == None or res[0] == None:
        return "", 404
    hash = res[0]
    return redirect(f"{request.host_url}hashed/{hash}.bsp.bz2", code=302)
@app.route('/maps/<mapname>.bsp.bz2')
def map_mapname(mapname):
    pass
@app.route('/m2/<mapname>.bsp.bz2')
def m2_mapname(mapname):
    pass
@app.route('/h2/<hash>/<mapname>.bsp.bz2')
def h2_hash_mapname(hash, mapname):
    if not Path(f"{HASHFOLDER}/{hash}.bsp.bz2").is_file():
        return "", 404
    return "", 200, {"Content-Type": "application/x-bzip", "X-Accel-Redirect": f"{HASHFOLDER}/{hash}.bsp.bz2"}
@app.route('/hashed/<hash>/<mapname>.bsp.bz2')
def hashed_hash_mapname(hash, mapname):
    if not Path(f"{HASHFOLDER}/{hash}.bsp.bz2").is_file():
        return "", 404
    return "", 200, {"Content-Type": "application/x-bzip", "X-Accel-Redirect": f"{HASHFOLDER}/{hash}.bsp.bz2"}
@app.route('/hashed/<hash>.bsp.bz2')
def hashed_hash(hash, mapname):
    if not Path(f"{HASHFOLDER}/{hash}.bsp.bz2").is_file():
        return "", 404
    return "", 200, {"Content-Type": "application/x-bzip", "X-Accel-Redirect": f"{HASHFOLDER}/{hash}.bsp.bz2"}
"""

# /hashed/hash.bsp.bz2 || /maps/mapname.bsp.bz2 || /m2/mapname.bsp.bz2  || /mapsredir/mapname.bsp.bz2
@app.route('/<folder>/<thing>.bsp.bz2')
def route2(folder, thing):
    if folder == "hashed":
        hash = thing
        if not Path(f"{HASHFOLDER}/{hash}.bsp.bz2").is_file():
            return "", 404
        #loghash(thing, "", folder)
        # TODO: redirect to moonbucket.fastdl.me?
        return "", 200, {"Content-Type": "application/x-bzip", "X-Accel-Redirect": f"{HASHFOLDER}/{hash}.bsp.bz2"}
    elif folder == "maps" or folder == "mapsredir" or folder == "m2":
        if smells_like_sha1(thing):
            hash = thing.lower()
        else:
            cur = get_db().cursor()
            cur.execute("SELECT sha1, MAX(filesize_bz2) fbz2 FROM maps_canon WHERE mapname = ?", (thing.lower(),))
            res = cur.fetchone()
            if res == None or res[0] == None:
                #log404(mapname, "bz2")
                return "", 404
            hash = res[0]
        #logmap(hash, mapname, folder)
        if folder == "mapsredir":
            return redirect(f"{request.host_url}hashed/{hash}.bsp.bz2", code=302)
        else:
            return "", 200, {"Content-Type": "application/x-bzip", "X-Accel-Redirect": f"{HASHFOLDER}/{hash}.bsp.bz2"}
    return "", 404

# /h2/123123/mapname.bsp.bz2 || /hashed/123123/mapname.bsp.bz2
@app.route('/<folder>/<hash>/<mapname>.bsp.bz2')
def route3(folder, hash, mapname):
    if not Path(f"{HASHFOLDER}/{hash}.bsp.bz2").is_file():
        return "", 404
    #logmap(hash, mapname, folder)
    return "", 200, {"Content-Type": "application/x-bzip", "X-Accel-Redirect": f"{HASHFOLDER}/{hash}.bsp.bz2"}

@app.route('/maps/<mapname>.bsp')
def bsp404(mapname):
    #log404(mapname, "bsp")
    return "", 404

if __name__ == '__main__':
    app.run(host='127.0.0.1', port=55155)
