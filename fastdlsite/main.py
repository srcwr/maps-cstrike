from flask import Flask,redirect,g,abort,request
import sqlite3

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

@app.route('/<mapsfolder>/<mapname>.bsp.bz2')
def AAAAA(mapsfolder, mapname):
    if mapsfolder == "maps" or mapsfolder == "mapsredir":
        return yo(mapsfolder, mapname, False)
    else:
        return yo(mapsfolder, mapname, True)

def smells_like_sha1(s):
    try:
        x = int(s, 16)
        return len(s) == 40
    except:
        return False

def yo(mapsfolder, mapname, xaccel):
    if smells_like_sha1(mapname):
        maphash = mapname.lower()
    else:
        cur = get_db().cursor()
        cur.execute("SELECT sha1, MAX(filesize_bz2) fbz2 FROM maps_canon WHERE mapname = ?", (mapname.lower(),))
        res = cur.fetchone()
        if res == None or res[0] == None:
            redirurl = request.headers.get("redirurl")
            if redirurl == None or redirurl == "":
                return "", 404
            else:
                return redirect(f"{redirurl}/maps/{mapname}.bsp.bz2", code=302)
        maphash = res[0]

    if xaccel:
        return "", 200, {"Content-Type": "application/x-bzip", "X-Accel-Redirect": f"/hashedyo/{maphash}.bsp.bz2"}

    return redirect(f"https://mainr2.fastdl.me/hashed/{maphash}.bsp.bz2", code=302)

if __name__ == '__main__':
    app.run(host='127.0.0.1', port=55155)
