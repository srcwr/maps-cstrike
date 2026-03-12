#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"/unprocessed

(grep -hoP '\/files\/.*?\.zip' <(curl --proxy socks5h://ca-mtr-wg-socks5-302.relays.mullvad.net:1080 https://ksf.surf/maps) | sort -u | sed 's|/files/||; s|\.zip$||') > ksf.surf_maps-list.txt

git reset
git add ksf.surf_maps-list.txt
git -c user.name=srcwrbot -c user.email=bot@srcwr.com commit -m "$(date +%Y%m%d%H%M) - ksf.surf/maps update"
