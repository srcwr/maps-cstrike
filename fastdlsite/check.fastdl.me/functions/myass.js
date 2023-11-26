// SPDX-License-Identifier: WTFPL

export async function onRequestPost(ctx) {
    const obj = await ctx.env.CHECK_FASTDL_BUCKET.get("S U B M I S S I O N S.html");
    return new Response(obj.body);
}
