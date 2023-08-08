// SPDX-License-Identifier: WTFPL
// Copyright 2023 rtldg <rtldg@protonmail.com>
// This file is part of fastdl.me (https://github.com/srcwr/maps-cstrike/)

export async function onRequestPost(ctx) {
    const obj = await ctx.env.CHECK_FASTDL_BUCKET.get("S U B M I S S I O N S.html");
    return new Response(obj.body);
}
