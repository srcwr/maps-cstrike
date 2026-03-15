-- SPDX-License-Identifier: WTFPL

-- docs: https://redbean.dev/

local re = require("re")
local sqlite3 = require("lsqlite3")

local function smells_like_sha1(s)
	return #s == 40 and s:match("^[0-9a-fA-F]+$") ~= nil
end

local MAPS_RE = re.compile([[^/(maps|mapsredir)/(.*)\.bsp\.bz2$]])

local DB_PATH = path.exists("../../processed/maps-lite.db") and "../../processed/maps-lite.db" or "/data/maps-lite.db"

--[[
-- https://stackoverflow.com/a/27028488
local function dump(o)
	if type(o) == 'table' then
		local s = '{ '
		for k,v in pairs(o) do
			if type(k) ~= 'number' then k = '"'..k..'"' end
			s = s .. '['..k..'] = ' .. dump(v) .. ','
		end
		return s .. '} '
	else
		return tostring(o)
	end
end
]]

SetLogLevel(kLogWarn)

function OnHttpRequest()
	local _, mapsfolder, mapname = MAPS_RE:search(GetPath())
	if mapname then
		mapname = string.lower(mapname)
		local maphash = ""

		if smells_like_sha1(mapname) then
			maphash = mapname
		else
			local db = assert(sqlite3.open(DB_PATH, sqlite3.OPEN_READONLY))
			local stmt = assert(db:prepare("SELECT sha1, MAX(filesize_bz2) fbz2 FROM maps_canon WHERE mapname = ?"))
			assert(stmt:bind_values(mapname))
			for row in stmt:rows() do
				--print(dump(row))
				if row[1] then
					maphash = row[1]
				end
			end
			stmt:finalize()
			db:close()

			if maphash == "" then
				SetStatus(404)
				return
			end
		end

		SetStatus(302)
		SetHeader("Location", "https://mainr2.fastdl.me/hashed/"..maphash..".bsp.bz2")
		return
	end

	SetStatus(404)
end
