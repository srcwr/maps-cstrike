// SPDX-License-Identifier: WTFPL

function validURL(str) {
	try {
		const url = new URL(str);
		return url.protocol == "http:" || url.protocol == "https:";
	} catch (e) {
		return false;
	}
}

// https://developers.cloudflare.com/workers/runtime-apis/html-rewriter/
/*
class BspAnchorFinder {
	seen = {};
	output = "";
	constructor(everything, url) {
		this.everything = everything;
		this.url = url;
	}
	element(element) {
		//const href = element.getAttribute("href");
		//console.log(`href = '${href}'`);
		//const name = href.trim().slice(0, -8).toLowerCase().replace(".", "_").replace(" ", "_");
		const name = element.Content.trim().slice(0, -8).toLowerCase().replace(".", "_").replace(" ", "_");
		console.log(`name = ${name}`);
		if (this.seen[name]) return;
		if (this.everything[name]) return;
		this.seen[name] = true;
		this.output += `${target}${x}\n`;
	}
}
*/

export async function onRequestPost(ctx) {
	let everything;
	try {
		const resp = await fetch("https://venus.fastdl.me/mapnames_and_filesizes.json");
		if (!resp.ok) return new Response(`failed to fetch ${resp.url}`, {status: 500});
		everything = await resp.json();
	} catch (e) {
		return new Response(`failed to read fetch mapnames_and_filesizes.json as json...`, {status: 500});
	}

	const target = (await ctx.request.text()).trim();
	if (target.length < 13) return new Response("url too short...\n", {status: 400});
	if (!validURL(target)) return new Response("invalid url\n", {status: 400});

	const resp = await fetch(target, {
		headers: {
			"Referer": ctx.request.url,
		},
	});

	if (!resp.ok) return new Response(`server returned ${resp.status}\n`, {status: 500});

	/*
	const handler = new BspAnchorFinder(everything, target);
	new HTMLRewriter().on(`a[href$=".bsp.bz2"]`, handler).transform(resp);

	if (handler.output == "") handler.output = "No unique mapnames!\n";
	return new Response(handler.output);
	*/

	const pageText = await resp.text();

	const matches = pageText.match(/"([^"]+\.bsp\.bz2)"/g);

	const seen = {};
	let output = "";
	matches.forEach((x, i) => {
		const name = x
			.trim() // obv
			.slice(1, -9) // turn ">bhop_badges.bsp.bz2<" into "bhop_badges"
			.toLowerCase() // only lowercase or else
			.replace(".", "_").replace(" ", "_"); // bcz
		if (seen[name]) return;
		if (everything[name]) return;
		seen[name] = true;
		output += `${target}${x.slice(1,-1)}\n`;
	});
	if (output == "") output = "No unique mapnames!\n";
	return new Response(output);
}
