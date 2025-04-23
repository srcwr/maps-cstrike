call npx --yes wrangler r2 object put venus/mapnames_and_filesizes.json --file=processed/check.fastdl.me/_thing.json --content-type "application/json" --remote
set /p PURGETOKEN=<..\secretpurge
curl -X POST "https://api.cloudflare.com/client/v4/zones/1aa75e18589c3649abe7da1eb740bf46/purge_cache" ^
	-H "Authorization: Bearer %PURGETOKEN%" ^
	-H "Content-Type: application/json" ^
	--data "{\"files\":[\"https://venus.fastdl.me/mapnames_and_filesizes.json\"]}"
