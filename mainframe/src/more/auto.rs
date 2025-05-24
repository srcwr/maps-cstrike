// SPDX-License-Identifier: WTFPL
// Copyright 2025 rtldg <rtldg@protonmail.com>

use std::sync::Arc;

use tokio::task::JoinSet;

use crate::{Bsps, SETTINGS, more};

pub(crate) async fn run(bsps: Option<Arc<Bsps>>, now: jiff::Timestamp) -> anyhow::Result<()> {
	let dumped = Arc::new(more::dumper::dumpher().await?);
	let bsps = bsps.unwrap_or(dumped);

	let mut set = JoinSet::new();
	set.spawn_blocking({
		let bsps = Arc::clone(&bsps);
		move || more::original_mapname::run(&bsps)
	});
	set.spawn_blocking({
		let bsps = Arc::clone(&bsps);
		move || more::timestamper::run(Some(&bsps))
	});
	set.spawn_blocking({
		let bsps = Arc::clone(&bsps);
		move || more::lump_checksummer::run(&bsps)
	});
	// the vscripter is also handled by dumper
	/*
	set.spawn_blocking({
		let bsps = Arc::clone(&bsps);
		move ||
		more::vscripter::run(&bsps)
	});
	*/
	while let Some(res) = set.join_next().await {
		res??;
	}

	tokio::process::Command::new("git")
		.current_dir(&SETTINGS.dir_maps_cstrike_more)
		.args([
			"add",
			"entitiesgz",
			"filelist",
			"ignore_pak.csv",
			"ignore.csv",
			"original_mapname.csv",
			"timestamps.csv",
			"lump_checksums.csv",
			"vscript_probably.csv",
		])
		.status()
		.await?;

	let now = now.strftime("%Y%m%d%H%M").to_string();

	tokio::process::Command::new("git")
		.current_dir(&SETTINGS.dir_maps_cstrike_more)
		.args([
			"-c",
			"user.name=srcwrbot",
			"-c",
			"user.email=bot@srcwr.com",
			"commit",
			"-m",
			&now,
		])
		.status()
		.await?;
	tokio::process::Command::new("git")
		.current_dir(&SETTINGS.dir_maps_cstrike_more)
		.args(["push", &SETTINGS.git_origin])
		.status()
		.await?;

	Ok(())
}
