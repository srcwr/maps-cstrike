// SPDX-License-Identifier: WTFPL

// wrangler pages publish --project-name check-fastdl --branch main .

export async function onRequestPost(ctx) {
    const now  = new Date().toISOString().replace('T', ' ').slice(0, -5); // lol

    const whresp = await fetch(ctx.env.WEBHOOKURL, {
        method: 'POST',
        headers: {
            'Accept': 'application/json',
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            username: "check.fastdl.me",
            avatar_url: "",
            content: now /*+ ' - ' + ctx.request.cf.country ' - ' + ctx.request.headers.get("X-Real-IP")*/ + '\n```\n' + (await ctx.request.text()) + '\n```',
            tts: false,
            flags: 4, // SUPPRESS_EMBEDS
            allowed_mentions: {
                parse: []
            },
        }),
    });

    if (!whresp.ok)
        return new Response('{}', {status: 500});

    let submissions = await ctx.env.CHECK_FASTDL_BUCKET.get("S U B M I S S I O N S.html");
    await ctx.env.CHECK_FASTDL_BUCKET.put("S U B M I S S I O N S.html",
        `<li>${now}<br><i>Not yet viewed</i></li>\n` + await submissions.text())

    return new Response('{"abc": 123}');
}
