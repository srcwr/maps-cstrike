// SPDX-License-Identifier: WTFPL

export async function onRequestPost(ctx) {
	let everything;
	try {
		const resp = await fetch("https://venus.fastdl.me/mapnames_and_filesizes.json");
		if (!resp.ok) return new Response(`failed to fetch ${resp.url}`, {status: 500});
		everything = await resp.json();
	} catch (e) {
		return new Response(`failed to read fetch mapnames_and_filesizes.json as json...`, {status: 500});
	}

    var output = "";
    (await ctx.request.text()).split('\n').forEach((x, i) => {
        if (x.trim() == "") return;
        const length = +x.substring(0, x.indexOf(' '));
        const origname = x.substring(x.indexOf(' ') + 1).slice(0, -4);
        const name = origname.toLowerCase().replace(".", "_").replace(" ", "_");
        if (!everything[name] || !everything[name].includes(length))
            output += `UNIQUE! ${origname}\n`;
    });
    if (output == "") output = "No unique filename&filesize combinations!\n";
    return new Response(`Comparing against maps from https://fastdl.me\n\n${output}`);
}
