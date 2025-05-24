// SPDX-License-Identifier: WTFPL
// Copyright 2025 rtldg <rtldg@protonmail.com>

use std::sync::Arc;

use tokio::task::JoinSet;

use crate::{Bsps, base, cloudflare, more};

pub(crate) async fn run(
	now: jiff::Timestamp,
	bz2_upload_tasks: Option<JoinSet<anyhow::Result<()>>>,
	new_bsps: Option<Bsps>,
) -> anyhow::Result<()> {
	let mut bz2_upload_tasks = if let Some(bz2_upload_tasks) = bz2_upload_tasks {
		bz2_upload_tasks
	} else {
		let mut syncset = JoinSet::new();
		syncset.spawn(async { cloudflare::sync_r2_bz2s().await });
		syncset
	};

	let maps_cstrike_more_auto = tokio::task::spawn({
		let new_bsps = new_bsps.map(Arc::new);
		async move { more::auto::run(new_bsps, now).await }
	});
	let venus = tokio::spawn(more::venus::upload());

	base::process::run().await?;

	let mut node_transfers = base::nodes::transfer();

	while let Some(res) = node_transfers.join_next().await {
		if let Err(e) = res? {
			eprintln!("failed to transfer to node\n{e:?}");
			// TODO: (low) maybe abort?
		}
	}

	while let Some(res) = bz2_upload_tasks.join_next().await {
		if let Err(e) = res? {
			eprintln!("failed to upload a bz2\n{e:?}");
			// TODO: (low) maybe abort?
		}
	}

	if let Err(e) = maps_cstrike_more_auto.await.unwrap() {
		eprintln!("maps_cstrike_more_auto failed\n{e:?}");
		// TODO: (low) maybe abort?
	}
	if let Err(e) = venus.await? {
		eprintln!("venus failed\n{e:?}");
	}

	if let Err(e) = cloudflare::purge_cache(None).await {
		eprintln!("purging cache failed\n{e:?}");
	}

	Ok(())
}
