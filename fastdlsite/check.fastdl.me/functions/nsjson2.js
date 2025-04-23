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

    const json = await ctx.request.json();
    let unique = [];
    json.forEach((x, i) => {
        const length = x["Length"];
        const origname = x["Name"].slice(0, -4);
        const name = origname.toLowerCase().replace(".", "_").replace(" ", "_");
        if (!everything[name] || !everything[name].includes(length))
            unique.push(x["Name"].slice(0, -4));
    });
    return new Response(JSON.stringify({unique: unique}));
}
