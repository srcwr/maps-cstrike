# hashes & filesizes of many map/bsp files for Counter-Strike: Source

Used to help with archiving maps & hosting a big fastdl.

Originally intended for bhop/surf/xc/kz/trikz maps... and then it spiraled out of control...
- movement/skill gamemode maps get the most love though... there's around 2000 rows of duplicate mapnames excluding bhop/surf/xc/kz/trikz/etc so... that will probably never be proactively dealt with for `/maps/` / `canon.csv`....
	- feel free to request setting the correct version of a map though

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



SELECT m.mapname, m.sha1 FROM maps_canon m
JOIN (SELECT sha1, COUNT(*) c FROM maps_canon GROUP BY sha1 HAVING c > 1) t
ON m.sha1 = t.sha1

ORDER BY m.sha1 ASC, m.mapname ASC
```
