// SPDX-License-Identifier: WTFPL

export async function onRequestPost(ctx) {
    const frick = new URL(ctx.request.url);
    frick.pathname = "_thing.json";
    const everything = await (await ctx.env.ASSETS.fetch(frick)).json();

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
