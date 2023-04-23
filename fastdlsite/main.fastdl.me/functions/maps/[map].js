// SPDX-License-Identifier: MIT
// Copyright 2021-2023 Cloudflare, Inc.
// Copyright 2023 rtldg <rtldg@protonmail.com>

// wrangler pages publish --project-name mfdl --branch master .

export async function onRequest(context) {
    const file = context.params.map;

    if (file.endsWith(".bsp")) {
        // hit nginx server with .bsp 404
        return new Response(null, {status: 404});
    }

    const method = context.request.method;

    if (method != "GET" && method != "HEAD") return new Response(null, {status: 405});

    const match = /^(.+).bsp.bz2$/.exec(file);
    if (match == null) return new Response(null, {status: 404});

    const mapname = match[1].toLowerCase();

    const hash = await context.env.DB.prepare(
        "SELECT sha1, MAX(filesize_bz2) fbz2 FROM maps WHERE mapname = ?"
    ).bind(mapname).first("sha1");

    if (hash == null) return new Response(null, {status: 404});

    const url = new URL(context.request.url);
    const mapsredir = url.pathname.startsWith("/mapsredir/");

    if (mapsredir) return Response.redirect(`https://mainr2.fastdl.me/hashed/${hash}.bsp.bz2`, 302);

    const range = (method == "GET") ? parseRange(context.request.headers.get('range')) : null;
    const object = (method == "GET")
      ? await context.env.FASTDL_BUCKET.get(`hashed/${hash}.bsp.bz2`, { range, onlyIf: context.request.headers })
      : await context.env.FASTDL_BUCKET.head(`hashed/${hash}.bsp.bz2`);

    if (object === null) return new Response(null, {status: 404});

    const headers = new Headers();
    object.writeHttpMetadata(headers);
    headers.set("etag", object.httpEtag);

    if (range) headers.set("content-range", `bytes ${range.offset}-${range.end}/${object.size}`);

    const status = (method == "HEAD") ? 200 : (object.body ? (range ? 206 : 200) : 304);
    return new Response(object.body, {headers, status});
}

// https://developers.cloudflare.com/r2/examples/demo-worker/
function parseRange(encoded) {
    if (encoded === null) return;
    const parts = encoded.split("bytes=")[1]?.split("-") ?? [];
    if (parts.length !== 2) {
        throw new Error('Not supported to skip specifying the beginning/ending byte at this time');
    }
    return {
        offset: Number(parts[0]),
        end:    Number(parts[1]),
        length: Number(parts[1]) + 1 - Number(parts[0]),
    };
}
