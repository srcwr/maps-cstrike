// SPDX-License-Identifier: WTFPL
// Copyright 2025 rtldg <rtldg@protonmail.com>

use std::{
	collections::{BTreeMap, HashMap, btree_map},
	sync::{Arc, LazyLock, Mutex},
	time::Duration,
};

use itertools::Itertools;
use tokio::task::JoinSet;

use crate::{Bsps, CLIENT, SETTINGS, base, cloudflare, discord, gamebanana::discordbot, hex_to_hash, normalize_mapname};

use super::types::{ARecords1, ApiV11Mod, ApiV11ModIndex};

#[derive(PartialEq)]
enum LoopControl {
	Restart(f32),
	Pass,
}

async fn get_index_records() -> Vec<ARecords1> {
	let start_page = SETTINGS.gb_itemoffset / SETTINGS.gb_perpage + 1;
	let last_page = SETTINGS.gb_numtofetch.get() / SETTINGS.gb_perpage + start_page;
	let mut perpage = SETTINGS.gb_perpage.get();

	if SETTINGS.gb_itemoffset == 0 {
		// this stupid little bitch is used to walk up from 5 to 50 items per index. THis is because of annoying gamebanana caching...
		static WALK: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(SETTINGS.gb_perpage.get()));
		let mut x = WALK.lock().unwrap();
		perpage = *x;
		if *x == SETTINGS.gb_maxperpage.get() {
			*x = SETTINGS.gb_perpage.get();
		} else {
			*x += 1;
		}
	}

	let mut records = vec![];
	for page in start_page..last_page {
		'categories: for category in &SETTINGS.gb_categories {
			tokio::time::sleep(Duration::from_secs(1)).await;

			let url = format!(
				"https://gamebanana.com/apiv11/Mod/Index?_nPerpage={perpage}&_sSort=Generic_LatestModified&_aFilters%5BGeneric_Category%5D={category}&cachebuster={}{}",
				rand::random::<u64>(),
				if page == 1 { String::new() } else { format!("&_nPage={page}") },
			);
			//println!("fetching {url}");
			let resp = match CLIENT.get(&url).send().await {
				Ok(resp) => resp,
				Err(e) => {
					eprintln!("failed to fetch {url}\n{e:?}");
					continue 'categories;
				}
			};
			let resp = match resp.json::<ApiV11ModIndex>().await {
				Ok(resp) => resp,
				Err(e) => {
					eprintln!("received invalid json from {url}\n{e:?}");
					continue 'categories;
				}
			};

			records.extend(resp._aRecords);
		}
	}

	records
}

async fn update_modified_times(modified_times: &mut Arc<crate::csv::ModifiedTimes>) -> anyhow::Result<LoopControl> {
	let records = get_index_records().await;

	let mut updated = false;
	{
		let modified_times = Arc::get_mut(modified_times).unwrap();

		for record in records {
			if let Some(found) = modified_times.get(&record._idRow) {
				if record._tsDateModified <= found.lastmodified {
					continue;
				}
			}
			let _ = modified_times.insert(
				record._idRow,
				crate::csv::ModifiedTimesRow {
					modid: record._idRow,
					lastmodified: record._tsDateModified,
					checked: false,
				},
			);
			updated = true;
		}
	}

	if updated {
		crate::csv::write_modified_times(Arc::clone(modified_times)).await?;
		Ok(LoopControl::Restart(1.0))
	} else {
		Ok(LoopControl::Pass)
	}
}

async fn fetch_mod(
	downloads: &mut Arc<crate::csv::Downloads>,
	modified_times: &mut Arc<crate::csv::ModifiedTimes>,
) -> anyhow::Result<LoopControl> {
	let find_next_mod = || {
		for (modid, row) in modified_times.iter() {
			if !row.checked {
				return Some(*modid);
			}
		}
		None
	};
	let Some(modid) = find_next_mod() else {
		return Ok(LoopControl::Pass);
	};

	let url = format!(
		"https://gamebanana.com/apiv11/Mod/{modid}?_csvProperties=_aFiles&cachebuster={}",
		rand::random::<u64>()
	);
	println!("fetching {url}");
	let files = CLIENT.get(&url).send().await;
	let files = match files {
		Ok(files) => files,
		Err(e) => {
			eprintln!("failed to fetch {url}\n{e:?}");
			return Ok(LoopControl::Restart(SETTINGS.gb_wait_regular));
		}
	};
	let files = match files.json::<ApiV11Mod>().await {
		Ok(files) => files._aFiles,
		Err(e) => {
			eprintln!("failed to parse json file list from {url}\n{e:?}");
			return Ok(LoopControl::Restart(SETTINGS.gb_wait_regular));
		}
	};

	{
		let modified_times = Arc::get_mut(modified_times).unwrap();
		modified_times.get_mut(&modid).unwrap().checked = true;
	}
	crate::csv::write_modified_times(Arc::clone(modified_times)).await?;

	let mut inserted_new_row = false;

	if let Some(files) = files {
		let downloads = Arc::get_mut(downloads).unwrap();
		for file in files {
			if let btree_map::Entry::Vacant(vacant_entry) = downloads.entry((modid, file._idRow)) {
				vacant_entry.insert(crate::csv::DownloadsRow {
					modid,
					downloadid: file._idRow,
					filename: file._sFile,
					downloaded: false,
					processed: false,
				});
				inserted_new_row = true
			}
		}
	}

	if inserted_new_row {
		crate::csv::write_downloads(Arc::clone(downloads)).await?;
	}

	Ok(LoopControl::Restart(1.0))
}

async fn download_item(downloads: &mut Arc<crate::csv::Downloads>) -> anyhow::Result<LoopControl> {
	let find_next_download = || {
		for ((_modid, _downloadid), row) in downloads.iter() {
			if !row.downloaded {
				return Some(row.clone());
			}
		}
		None
	};
	let Some(row) = find_next_download() else {
		return Ok(LoopControl::Pass);
	};

	let outputfilename = format!("{}_{}_{}", row.modid, row.downloadid, row.filename);
	let _ = discord::webhook(
		true,
		&format!("new download at https://gamebanana.com/mods/{} `{outputfilename}`", row.modid),
	)
	.await;

	let link = format!("https://gamebanana.com/dl/{}", row.downloadid);
	println!("downloading {link}");
	// TODO: (low) progress bar...
	let resp = match CLIENT.get(&link).send().await {
		Ok(resp) => resp,
		Err(e) => {
			eprintln!("failed to download {link}\n{e:?}");
			let _ = discord::webhook(true, &format!("{} on {link}", e.status().unwrap_or_default().as_u16())).await;
			return Ok(LoopControl::Restart(SETTINGS.gb_wait_regular));
		}
	};

	let resp = match resp.error_for_status() {
		Ok(resp) => resp,
		Err(e) => {
			eprintln!("failed to download2 {link}\n{e:?}");
			let _ = discord::webhook(true, &format!("{} on {link}", e.status().unwrap_or_default().as_u16())).await;
			return Ok(LoopControl::Restart(SETTINGS.gb_wait_regular));
		}
	};

	let bytes = match resp.bytes().await {
		Ok(bytes) => bytes,
		Err(e) => {
			eprintln!("stream died during {link} download?\n{e:?}");
			let _ = discord::webhook(
				true,
				&format!(
					"failed to download {link} (stream died?) ({})",
					e.status().unwrap_or_default().as_u16()
				),
			)
			.await;
			return Ok(LoopControl::Restart(SETTINGS.gb_wait_regular));
		}
	};

	tokio::fs::write(SETTINGS.dir_gamebanana_scrape.join(&outputfilename), &bytes).await?;

	{
		let downloads = Arc::get_mut(downloads).unwrap();
		let entry = downloads.get_mut(&(row.modid, row.downloadid)).unwrap();
		entry.downloaded = true;
	}
	crate::csv::write_downloads(Arc::clone(downloads)).await?;

	Ok(LoopControl::Restart(0.0))
}

async fn process_item(
	downloads: &mut Arc<crate::csv::Downloads>,
	new_bsps: &mut Bsps,
	bz2_upload_tasks: &mut JoinSet<anyhow::Result<()>>,
	now: &mut Option<jiff::Timestamp>,
) -> anyhow::Result<LoopControl> {
	let find_next = || {
		for ((_modid, _downloadid), row) in downloads.iter() {
			if row.downloaded && !row.processed {
				return Some(row.clone());
			}
		}
		None
	};
	let Some(row) = find_next() else {
		return Ok(LoopControl::Pass);
	};

	if now.is_none() {
		*now = Some(jiff::Timestamp::now());
	}

	let shortnow = now.as_ref().unwrap().strftime("%Y%m%d%H%M").to_string();
	let prettynow = now.as_ref().unwrap().strftime("%Y-%m-%d %H:%M").to_string();
	let prettynow = compact_str::CompactString::from(prettynow);

	let outputfilename = format!("{}_{}_{}", row.modid, row.downloadid, row.filename);
	let outputdir = SETTINGS.dir_gamebanana_auto.join(&shortnow).join(&outputfilename);

	let status = tokio::process::Command::new("7z")
		.args([
			"x",
			"-y",
			SETTINGS.dir_gamebanana_scrape.join(&outputfilename).to_str().unwrap(),
			&format!("-o{}", outputdir.to_str().unwrap()),
		])
		.status()
		.await?;

	anyhow::ensure!(status.success(), "failed to extract {outputfilename}");

	let mut newly_hashed = base::mapshasher::run(
		SETTINGS.dir_maps_cstrike.join("unprocessed/gamebanana-x-automatic.csv"),
		base::mapshasher::Mode::Automatic,
		&outputdir,
		false,
		false,
		true,
	)
	.await?;
	newly_hashed.sort();

	// start uploading .bsp.bz2 files to r2...
	for sha1 in newly_hashed.iter().map(|r| r.sha1.clone()).unique() {
		let localpath = SETTINGS.dir_hashed.join(format!("{sha1}.bsp.bz2"));
		let remotepath = format!("hashed/{sha1}.bsp.bz2");
		let tries = 5;
		bz2_upload_tasks.spawn(async move {
			for i in 0..tries {
				match cloudflare::r2_upload(&localpath, "hashed", &remotepath, "application/x-bzip").await {
					Ok(()) => {
						return Ok(());
					}
					Err(e) => {
						eprintln!("failed to upload {sha1}.bsp.bz2 to r2 attempt #{}\n{e:?}", i + 1);
						if i != tries - 1 {
							tokio::time::sleep(Duration::from_secs(3)).await;
						}
					}
				}
			}
			anyhow::bail!("failed to upload {sha1}.bsp.bz2 after {tries} tries...");
		});
	}

	// do some automatic canonization stuff....
	// TODO: (low) doesn't really handle new maps stealing names...
	// TODO: (higher) okay, maybe that should be dealt with at some point lol..
	if !newly_hashed.is_empty() {
		for item in &newly_hashed {
			new_bsps.insert(hex_to_hash(&item.sha1));
		}

		let recently_added: Vec<_> = csv::Reader::from_path(SETTINGS.dir_maps_cstrike.join("recently_added.csv"))?
			.deserialize::<crate::csv::UnprocessedCsvRow>()
			.try_collect()?;

		let mut recently_added_csv = csv::Writer::from_writer(vec![]);
		for row in &newly_hashed {
			let mut row = row.clone();
			row.recently_added_note = Some("automated upload".to_string());
			row.datetime = Some(prettynow.clone());
			recently_added_csv.serialize(row)?;
		}

		let mut newly_hashed_lookup = HashMap::new();
		for row in &newly_hashed {
			let _ = newly_hashed_lookup
				.entry(normalize_mapname(&row.mapname))
				.or_insert(row.sha1.clone());
		}
		let mut needs_canonization = BTreeMap::new();
		for row in &recently_added {
			let mapname = normalize_mapname(&row.mapname);
			if let Some(sha1) = newly_hashed_lookup.get(&mapname) {
				needs_canonization.insert(mapname, sha1.clone());
			} else {
				recently_added_csv.serialize(row)?;
			}
		}

		tokio::fs::write(
			SETTINGS.dir_maps_cstrike.join("recently_added.csv"),
			recently_added_csv.into_inner().unwrap(),
		)
		.await?;

		if !needs_canonization.is_empty() {
			let mut canons = vec![];
			{
				let mut canon_csv = csv::Reader::from_path(SETTINGS.dir_maps_cstrike.join("canon.csv"))?;
				for row in canon_csv.deserialize::<crate::csv::CanonCsvRow>() {
					let row = row?;
					if !needs_canonization.contains_key(&row.mapname) {
						canons.push(row);
					}
				}
			}
			for (mapname, sha1) in needs_canonization {
				canons.push(crate::csv::CanonCsvRow {
					mapname,
					sha1,
					note: format!("automatic canonization {shortnow}"),
				});
			}
			canons.sort();
			let mut canon_csv = csv::Writer::from_writer(vec![]);
			for row in canons {
				canon_csv.serialize(row)?;
			}
			tokio::fs::write(SETTINGS.dir_maps_cstrike.join("canon.csv"), canon_csv.into_inner()?).await?;
		}
	}

	{
		let downloads = Arc::get_mut(downloads).unwrap();
		let entry = downloads.get_mut(&(row.modid, row.downloadid)).unwrap();
		entry.processed = true;
	}
	crate::csv::write_downloads(Arc::clone(downloads)).await?;

	Ok(LoopControl::Restart(0.0))
}

pub(crate) async fn run() -> anyhow::Result<()> {
	tokio::spawn(discordbot::lurk());

	let mut downloads = Arc::new(crate::csv::load_downloads().await?);
	let mut modified_times = Arc::new(crate::csv::load_modified_times().await?);

	let mut queued_now = None;
	let mut new_bsps = Bsps::new();
	let mut bz2_upload_tasks = JoinSet::new();

	let mut control = LoopControl::Pass;

	// this will process every possible item
	// then download every possible item
	// then fetch every possible mod
	// then update modified times
	// and if there's nothing new, then it commits & starts maps-cstrike-more-auto stuff...
	//
	// so what really happens is:
	// - there's new modified time from update_modified_times
	// - fetch_mod on the mod that had a new modified time
	// - download the new files on the mod
	// - process the downloaded files
	//
	// trickle-down economics or something idk

	loop {
		if let LoopControl::Restart(sleep_seconds) = control {
			if sleep_seconds > 0.0 {
				tokio::time::sleep(Duration::from_secs_f32(sleep_seconds)).await;
			}
		}

		control = process_item(&mut downloads, &mut new_bsps, &mut bz2_upload_tasks, &mut queued_now).await?;
		if control != LoopControl::Pass {
			continue;
		}

		control = download_item(&mut downloads).await?;
		if control != LoopControl::Pass {
			continue;
		}

		control = fetch_mod(&mut downloads, &mut modified_times).await?;
		if control != LoopControl::Pass {
			continue;
		}

		control = update_modified_times(&mut modified_times).await?;
		if control != LoopControl::Pass {
			continue;
		}

		if let Some(now) = queued_now.take() {
			let push_maps_cstrike = tokio::spawn(async move {
				let now = now.strftime("%Y%m%d%H%M").to_string();
				let _x = tokio::process::Command::new("git")
					.current_dir(&SETTINGS.dir_maps_cstrike)
					.args([
						"add",
						"recently_added.csv",
						"unprocessed/gamebanana-x-automatic.csv",
						"canon.csv",
						"gamebanana-downloads.csv",
						"gamebanana-modified-times.csv",
					])
					.status()
					.await
					.unwrap();
				let _x = tokio::process::Command::new("git")
					.current_dir(&SETTINGS.dir_maps_cstrike)
					.args([
						"-c",
						"user.name=srcwrbot",
						"-c",
						"user.email=bot@srcwr.com",
						"commit",
						"-m",
						&format!("{now} - automatic gamebanana"),
					])
					.status()
					.await
					.unwrap();
				tokio::process::Command::new("git")
					.current_dir(&SETTINGS.dir_maps_cstrike)
					.args(["push", &SETTINGS.git_origin])
					.status()
					.await
					.unwrap()
			});

			base::semimanual::run(
				now,
				Some(std::mem::take(&mut bz2_upload_tasks)),
				Some(std::mem::take(&mut new_bsps)),
			)
			.await?;

			if !push_maps_cstrike.await.unwrap().success() {
				eprintln!("failed to push maps-cstrike to github");
			}

			println!("\n\nrunning it back!\n\n");
		}

		control = LoopControl::Restart(SETTINGS.gb_wait_looped);

		// some bullshit to stop eating so much memory...
		let mut memory_set = JoinSet::new();
		for _ in 0..std::thread::available_parallelism()?.get() * 2 {
			memory_set.spawn_blocking(|| unsafe {
				std::thread::sleep(Duration::from_secs(1));
				libmimalloc_sys::mi_collect(true);
			});
		}
		let _ = memory_set.join_all().await;
		unsafe {
			libmimalloc_sys::mi_collect(true);
		}
	}
}
