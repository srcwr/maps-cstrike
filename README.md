# hashes & filesizes of many map/bsp files for Counter-Strike: Source

Used to help with archiving maps & hosting a big fastdl.

Originally intended for bhop/surf/xc/kz/trikz maps... and then it spiraled out of control...
- movement/skill gamemode maps get the most love though... there's around 2000 rows of duplicate mapnames excluding bhop/surf/xc/kz/trikz/etc so... that will probably never be proactively dealt with for `/maps/` / `canon.csv`....
	- feel free to request setting the correct version of a map though

## LICENSE
Code, .html files, .txt files, and datasets (CSVs) are licensed under the [DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE](http://www.wtfpl.net/txt/copying/) unless otherwise specified in the file.

## TODO
The gamebanana scraper (gamebanana-automatic.py & [this](https://github.com/srcwr/gamebanana-things)) is not the most robust. If the connection fails during a download then that entire queue will be thrown away, and to fix you have to delete all the recently downloaded archives. That's not too hard to fix but TODO. Also the gamebanana API endpoint is old, because documentation on endpoints suck, so we miss new archives when: it's off the first page or two, which happens when (1) an item is unprivated (2) an item is given a new file but not a new version. Those are caught with random manual multi-page rechecks for new files. One day these will surely:tm: be fixed.

## WHAT IS WHAT
- `_thing/`
	- Old scripts for quickly checking if someone's `maps` folder has anything unique to upload
- `fastdlsite/`
	- [`check.fastdl.me`](https://check.fastdl.me/)
		- Map submission form and commands to check your `maps` folder for unique things to upload
	- [`fastdl.me`](https://fastdl.me/)
		- The homepage
	- `fastdlpy/`
		- `99-cloudflared.conf`
			- some sysctls I put on to silence some cloudflared warnings
		- `fastdlpy.service`
			- a systemd service file for fastdlpy
		- `main.py`
			- The meat of the name-to-hash redirections for [main.fastdl.me](https://main.fastdl.me/)
		- `main2.py`
			- Currently unused but basically the same thing but instead for a server that is hosting the .bsp.bz2 itself
		- `requirements.txt`
			- pip requirements yada yada
		- `setup-venv.sh`
			- quicker venv setup so I don't have to remember every time
	- `fastdlpy_nocf/'
		- non-cloudflare version of fastdlpy. Could probably just be merged with fastdlpy and use environment-variables to configure it instead.
	- `log-counter/`
		- Unused nginx log 'parser' to get the count of each map downloaded in a day
	- [`main.fastdl.me`](https://main.fastdl.me/)
		- Mainly placeholder files for keeping the directory structure.
	- [`mainr2.fastdl.me`](https://mainr2.fastdl.me/)
		- More placeholder files.
	- `nginx/`
		- Most of the nginx website configuration for [main.fastdl.me](https://main.fastdl.me/)
	- `nginx_nocf/`
		- non-cloudflare version of the nginx configs. Old and won't be used.
	- `venus.fastdl.me/`
		- HTML files for the subdomain that serves [RawInput2BunnyhopAPE](https://github.com/rtldg/RawInput2BunnyhopAPE) `.bsp` checksums.
	- `compose.yml`
		- Docker compose file for spinning up a fastdl node.
	- `compose.yml`
		- Docker compose file for spinning up a non-cloudflare fastdl node.
	- `embedded-privacy-policy.html`
		- Basic minimal privacy policy that's appended to pages.
- `filters/`
	- CSV files for removing dupes, bad maps, etc from the fastdl.me `/maps/` page
- `processed/`
	- Built website code after you run `python process.py`
- `unprocessed/`
	- CSV files for all the map scrapes & dumps used by fastdl.me
- `canon.csv`
	- CSV with which hash to use for particular map names. Needed for maps that update without changing the name for example
- `gamebanana-automatic.py`
	- Automatic gamebanana downloader & uploader script
- `index_bottom.html`
	- A template used by `process.py` when generating map folder HTML index files
- `index_top.html`
	- A template used by `process.py` when generating map folder HTML index files
- `maps-hasher.py`
	- The map hasher & dumper-to-CSV-files part of adding new maps.
- `process.py`
	- The step to parse all the CSV files to produce the website files & sqlite db to put into `processed/`
- `recently_added.csv`
	- List of recently added map files that is put at the top of map folder HTML index files
- `shit.txt`
	- Where `map-hasher.py` warnings and errors are dumped for later reference
- `todo.txt`
	- A poorly named file...
- `transfer.sh`
	- Script to transfer `processed/` to a VPS


Sister repo at https://github.com/srcwr/maps-cstrike-more
- For every bsp: has a csv with the packed file list & a dump of the entity lump.
	- The entity lumps are compressed because the repo would've been 4 gigabytes otherwise...



"How do you determine what map should be in canon.csv?"
- First: don't use a version if it's corrupted/crashing :)
- Use the latest version from gamebanana if possible.
	- Otherwise `changelevel hash123123123` & `stripper_dump` on both maps.
	Then diff the entity lumps .cfg from that & check if one has a higher `mapversion` key at the top of the file.
		- Otherwise one of the cfgs might have things removed if someone edited the map so generally pick the one that doesn't have things removed. This one is subjective because somethings *should* be removed because they're stupid or break multiplayer but generally using the original release is the "right" move.
			- Otherwise otherwise otherwise pick a version of the map that's packed with textures.


Some useful sql things...
```sql
SELECT m.mapname, m.sha1 FROM maps_unfiltered m
JOIN (SELECT mapname, COUNT(*) c FROM maps_unfiltered GROUP BY mapname HAVING c > 1) t
ON m.mapname = t.mapname

ORDER BY m.mapname ASC, m.sha1 ASC



SELECT m.mapname, m.sha1 FROM maps_unfiltered m
JOIN (SELECT sha1, COUNT(*) c FROM maps_unfiltered GROUP BY sha1 HAVING c > 1) t
ON m.sha1 = t.sha1

ORDER BY m.sha1 ASC, m.mapname ASC



SELECT m.* FROM maps_canon m
JOIN (SELECT mapname, COUNT(*) c FROM maps_canon GROUP BY mapname HAVING c > 1) t
ON m.mapname = t.mapname

ORDER BY m.mapname ASC, m.sha1 ASC

# WHERE m.mapname LIKE '%bh%' OR m.mapname LIKE '%xc%' OR m.mapname LIKE '%kz%' OR m.mapname LIKE '%surf%' OR m.mapname LIKE '%trikz%'
# ^^^ You can also put this before the ORDER BY



SELECT m.mapname, m.sha1 FROM maps_canon m
JOIN (SELECT sha1, COUNT(*) c FROM maps_canon GROUP BY sha1 HAVING c > 1) t
ON m.sha1 = t.sha1

ORDER BY m.sha1 ASC, m.mapname ASC
```
