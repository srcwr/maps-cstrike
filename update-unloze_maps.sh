#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"/downloader-state

get_le_file() {
	curl --fail --proxy socks5h://ca-mtr-wg-socks5-302.relays.mullvad.net:1080 "https://fastdl-wrapper.unloze.com/api/list?prefix=$1%2Fmaps%2F" \
		| jq -r '.files[] | select(.key | endswith(".bsp.bz2")) | "\(.key) \(.size)"' \
		| sed "s|$1/maps/||; s|\\.bsp\\.bz2||" \
		| sort -u \
		> unloze-$1-list.csv
}

get_le_file "css_mg"
get_le_file "css_ze"
get_le_file "css_zr"

git reset
git add unloze-css_mg-list.csv unloze-css_ze-list.csv unloze-css_zr-list.csv
git -c user.name=srcwrbot -c user.email=bot@srcwr.com commit -m "$(date +%Y%m%d%H%M) - unloze fastdl update"
