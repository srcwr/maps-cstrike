from flask import Flask,redirect,g,abort
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

@app.route('/maps/<mapname>.bsp.bz2')
def AAAAA(mapname):
    cur = get_db().cursor()
    cur.execute("SELECT sha1, MAX(filesize_bz2) fbz2 FROM maps_canon WHERE mapname = ? LIMIT 1", (mapname,))
    res = cur.fetchone()
    if res == None or res[0] == None:
        #abort(404)
        return "", 404
    maphash = res[0]
    return redirect(f"https://mainr2.fastdl.me/hashed/{maphash}.bsp.bz2", code=302)

if __name__ == '__main__':
    app.run(host='127.0.0.1', port=55155)
