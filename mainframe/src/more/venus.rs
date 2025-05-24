// SPDX-License-Identifier: WTFPL
// Copyright 2025 rtldg <rtldg@protonmail.com>

use anyhow::Context;

use crate::SETTINGS;

pub(crate) async fn upload_lump_checksums() -> anyhow::Result<()> {
	crate::cloudflare::r2_upload(
		&SETTINGS.dir_maps_cstrike_more.join("lump_checksums.csv"),
		&SETTINGS.s3_bucket_venus,
		"lump_checksums.csv",
		"text/plain",
	)
	.await
}

pub(crate) async fn upload_mapnames_and_filesizes() -> anyhow::Result<()> {
	crate::cloudflare::r2_upload(
		&SETTINGS.dir_maps_cstrike.join("processed/mapnames_and_filesizes.json"),
		&SETTINGS.s3_bucket_venus,
		"mapnames_and_filesizes.json",
		"application/json",
	)
	.await
}

pub(crate) async fn upload() -> anyhow::Result<()> {
	let a = tokio::spawn(upload_lump_checksums());
	let b = tokio::spawn(upload_mapnames_and_filesizes());
	a.await?.context("upload_lump_checksums")?;
	b.await?.context("upload_mapnames_and_filesizes")?;
	Ok(())
}
