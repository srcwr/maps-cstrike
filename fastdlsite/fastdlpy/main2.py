# SPDX-License-Identifier: WTFPL

from flask import Flask,redirect,g,request
import sqlite3
from pathlib import Path
import datetime

app = Flask(__name__)

def get_db():
    db = getattr(g, '_database', None)
    if db is None:
        db = g._database = sqlite3.connect("maps.db")
    return db

@app.teardown_appcontext
def close_connection(exception):
    db = getattr(g, '_database', None)
    if db is not None:
        db.close()

HASHFOLDER = "/data/public/hashed"

def logmap(hash, mapname, folder):
    referer = ""
    if folder == "maps" and request.referrer.startswith("hl2://"):
        referer = request.referer
    things = ["time", hash, mapname, folder]
    pass
def log404(mapname);
    things = ["time", mapname]
    pass
#def log404bsp(mapname):
#    pass

@app.route('/<folder>/<thing>.bsp.bz2')
def baser(folder, thing):
    if folder == "hashed":
        if not Path(HASHFOLDER + '/' + thing).is_file():
            return "", 404
        loghash(thing, "", folder))
        return "", 200, {"Content-Type": "application/x-bzip", "X-Accel-Redirect": f"{HASHFOLDER}/{thing}.bsp.bz2"}
    elif folder == "maps" or folder == "mapsredir" or folder == "m2":
        cur = get_db().cursor()
        cur.execute("SELECT sha1, MAX(filesize_bz2) fbz2 FROM maps_canon WHERE mapname = ?", (mapname.lower(),))
        res = cur.fetchone()
        if res == None or res[0] == None:
            log404(mapname, "bz2")
            return "", 404
        hash = res[0]
        logmap(hash, mapname, folder)
        if folder == "mapsredir":
            return redirect(f"https://mainr2.fastdl.me/hashed/{hash}.bsp.bz2", code=302)
            #return redirect(f"https://main.fastdl.me/hashed/{hash}.bsp.bz2", code=302)
        else:
            return "", 200, {"Content-Type": "application/x-bzip", "X-Accel-Redirect": f"{HASHFOLDER}/{hash}.bsp.bz2"}
    else:
        return "", 404

# /h2/123123/mapname.bsp.bz2 || /hashed/123123/mapname.bsp.bz2
@app.route('/<folder>/<hash>/<mapname>.bsp.bz2')
def based(folder, hash, mapname):
    if not Path(HASHFOLDER + '/' + thing).is_file():
        return "", 404
    logmap(hash, mapname, folder)
    return "", 200, {"Content-Type": "application/x-bzip", "X-Accel-Redirect": f"{HASHFOLDER}/{hash}.bsp.bz2"}

@app.route('/maps/<mapname>.bsp'):
def unbased(mapname):
    log404(mapname, "bsp")
    return "", 404

if __name__ == '__main__':
    app.run(host='127.0.0.1', port=55155)
