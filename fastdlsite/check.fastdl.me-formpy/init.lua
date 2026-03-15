-- SPDX-License-Identifier: WTFPL

-- docs: https://redbean.dev/

local function envvars()
	local out = {}
	local environ = unix.environ()
	for i=1,#environ do
		local name, value = environ[i]:match("^(.-)=(.*)$")
		if name then
			out[name] = value
		end
	end
	return out
end

local WEBHOOKPATH = envvars()["WEBHOOKPATH"]

function OnHttpRequest()
	if GetPath() == WEBHOOKPATH then
		local j = DecodeJson(GetBody())
		local unixsec, _nanos = unix.clock_gettime()
		local y,m,d,h,m,s = unix.gmtime(unixsec)
		local f = "/data/forms/%.4d%.2d%.2d_%.2d%.2d%.2d.txt" % {y,m,d,h,m,s}
		if Barf(f, j["content"]) then
			SetHeader("Content-Type", "application/json")
			Write('{"yip":"pie"}')
		else
			SetStatus(503)
		end
		return
	end
	SetStatus(404)
end
