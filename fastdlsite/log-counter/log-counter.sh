#!/bin/sh

echo fastdl
rg --no-line-number '.* 302 .*/mapsredir/(.*)\.bsp\.bz2" "hl2' --only-matching --replace '$1' fastdl.me.log.* | sort | uniq -c
echo 404s
rg --no-line-number '.* 404 .*/maps/(.*)\.bsp" "hl2' --only-matching --replace '$1' 404maps.log.* | sort | uniq -c
