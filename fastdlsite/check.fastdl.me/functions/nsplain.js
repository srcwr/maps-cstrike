// SPDX-License-Identifier: WTFPL

export async function onRequestPost(ctx) {
    const frick = new URL(ctx.request.url);
    frick.pathname = "_thing.json";
    const everything = await (await ctx.env.ASSETS.fetch(frick)).json();

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
