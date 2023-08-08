// SPDX-License-Identifier: WTFPL
// Copyright 2023 rtldg <rtldg@protonmail.com>
// This file is part of fastdl.me (https://github.com/srcwr/maps-cstrike/)

// wrangler pages publish --project-name check-fastdl --branch main .

export async function onRequestPost(ctx) {
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
    return new Response('{"abc": 123}');
}
