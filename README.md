# hashes & filesizes of many map/bsp files for Counter-Strike: Source

Collected mainly through scraping "FastDL" webhosts.

## Usage in SQLite
TODO: Table keys so hash comparisons is faster...
```bash
$ sqlite3 new.db
CREATE TABLE maps (mapname TEXT NOT NULL, filesize INT NOT NULL, filesize_bz2 INT NOT NULL, sha1 TEXT NOT NULL);
CREATE INDEX mapname ON maps(mapname);
CREATE INDEX sha1 on maps(sha1);
.mode csv
.import processed/maps.csv maps
.exit
```
