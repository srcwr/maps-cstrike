# hashes & filesizes of many map/bsp files for Counter-Strike: Source

Used to help with archiving maps & hosting a big fastdl.

Originally intended for bhop/surf/xc/kz/trikz maps... and then it spiraled out of control...


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
