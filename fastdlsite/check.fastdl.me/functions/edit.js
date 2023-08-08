// SPDX-License-Identifier: WTFPL
// Copyright 2023 rtldg <rtldg@protonmail.com>
// This file is part of fastdl.me (https://github.com/srcwr/maps-cstrike/)

export async function onRequestPost(ctx) {
    if (ctx.request.headers.get("X-Videos") != ctx.env.EDIT_PASS)
        return new Response(':(', {status: 451});
    await ctx.env.CHECK_FASTDL_BUCKET.put("S U B M I S S I O N S.html", ctx.request.body)
    return new Response(':)');
}
