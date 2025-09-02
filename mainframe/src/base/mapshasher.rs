// SPDX-License-Identifier: WTFPL
// Copyright 2025 rtldg <rtldg@protonmail.com>

#[cfg(windows)]
use std::os::windows::fs::FileTimesExt;

use std::{collections::HashMap, io::Write, path::Path, sync::Arc};

use crate::{
	SETTINGS,
	csv::{ProcessedCsvRow, UnprocessedCsvRow},
	gamebanana::GamebananaID,
	normalize_mapname,
};

use sha1::Digest;
use tokio::io::AsyncWriteExt;

#[derive(Debug, PartialEq, Eq)]
pub enum Mode {
	Automatic,
	Manual,
}

pub(crate) async fn run<P1: AsRef<Path>, P2: AsRef<Path>>(
	outcsvpath: P1,
	mode: Mode,
	mapsfolder: P2,
	timestamp_fixer: bool,
	skip_existing_hash: bool,
	canon_clobber_check: bool,
) -> anyhow::Result<Vec<UnprocessedCsvRow>> {
	run_inner(
		outcsvpath.as_ref(),
		mode,
		mapsfolder.as_ref(),
		timestamp_fixer,
		skip_existing_hash,
		canon_clobber_check,
	)
	.await
}

async fn run_inner(
	outcsvpath: &Path,
	mode: Mode,
	mapsfolder: &Path,
	timestamp_fixer: bool,
	skip_existing_hash: bool,
	canon_clobber_check: bool,
) -> anyhow::Result<Vec<UnprocessedCsvRow>> {
	let mapsfolder = mapsfolder.canonicalize()?;

	if mode == Mode::Manual {
		if let Ok(metadata) = tokio::fs::metadata(&outcsvpath).await {
			if metadata.len() > 75 {
				anyhow::bail!("DON'T OVERWRITE THAT CSV! ({})", outcsvpath.display());
			}
		}
	}

	let mut existing_canon = HashMap::new();
	if canon_clobber_check {
		// TODO (low) canonClobber.csv
		let mut maps_index_csv =
			csv::Reader::from_path(SETTINGS.dir_maps_cstrike.join("processed/main.fastdl.me/maps_index.html.csv"))?;
		let headers = maps_index_csv.headers()?.clone();
		let mut raw_record = csv::StringRecord::new();
		while maps_index_csv.read_record(&mut raw_record)? {
			let row = raw_record.deserialize::<ProcessedCsvRow>(Some(&headers))?;
			let _ = existing_canon.insert(normalize_mapname(row.mapname), row.sha1.to_string());
		}
	}

	let mut existing_names = HashMap::<String, Option<GamebananaID>>::new();
	let mut existing_recents = HashMap::<String, Option<GamebananaID>>::new();
	if mode == Mode::Automatic {
		let mut hashed_csv = csv::Reader::from_path(
			SETTINGS
				.dir_maps_cstrike
				.join("processed/main.fastdl.me/hashed_index.html.csv"),
		)?;
		let headers = hashed_csv.headers()?.clone();
		let mut raw_record = csv::StringRecord::new();
		while hashed_csv.read_record(&mut raw_record)? {
			let row = raw_record.deserialize::<ProcessedCsvRow>(Some(&headers))?;
			let _ = existing_names.insert(
				normalize_mapname(row.mapname),
				row.url
					.split("https://gamebanana.com/mods/")
					.nth(1)
					.map(|id| id.parse().unwrap()),
			);
		}

		let mut recently_added_csv = csv::Reader::from_path(SETTINGS.dir_maps_cstrike.join("recently_added.csv"))?;
		// only allow clobbering mapnames for recently added gamebanana downloads....
		for row in recently_added_csv.deserialize::<UnprocessedCsvRow>() {
			let row = row?;
			let id = if let Some(note) = row.note {
				note.split('_').next().and_then(|s| s.parse().ok())
			} else {
				None
			};
			let mapname = normalize_mapname(&row.mapname);
			if let Some(existing_names_id) = existing_names.get(&mapname) {
				if id == *existing_names_id {
					existing_names.remove(&mapname);
				}
			}
			existing_recents.insert(mapname, id);
		}
	}

	let mut newly_hashed = vec![];
	let mut outcsv = csv::Writer::from_writer(vec![]);

	for entry in glob::glob(&mapsfolder.join("**/*.bsp").to_string_lossy())? {
		let entry = entry?.canonicalize()?;

		let metadata = tokio::fs::metadata(&entry).await?;
		if metadata.is_dir() {
			continue;
		}
		let filesize = metadata.len();
		if filesize == 0 {
			println!("==== empty file {}", entry.display());
			continue;
		}
		// the smallest known bsp (from gb) is 12k bytes
		if filesize < 5_000 {
			println!("==== file too small ({filesize} bytes) {}", entry.display());
			continue;
		}

		let content = Arc::new(tokio::fs::read(&entry).await?);

		let vbspver = &content[0..5];

		if vbspver != b"VBSP\x13" && vbspver != b"VBSP\x14" {
			if vbspver == b"VBSP\x15" {
				println!("==== skipping CS:GO map {}", entry.display());
			} else if vbspver == b"VBSP\x19" {
				println!("==== skipping Momentum Mod / Strata Source map {}", entry.display());
			} else if &vbspver[0..4] == b"\x1E\x00\x00\x00" || &vbspver[0..4] == b"\x1D\x00\x00\x00" {
				println!("==== skipping GoldSrc Map? {}", entry.display());
			} else {
				println!("==== not a CS:S map? {}", entry.display());
			}
			continue;
		}

		// arguably unnecessary to use spawn_blocking here...
		let digest = tokio::task::spawn_blocking({
			let content = Arc::clone(&content);
			move || const_hex::encode(sha1::Sha1::digest(content.as_slice()))
		})
		.await
		.unwrap();

		let hashedbsppath = SETTINGS.dir_hashed.join(format!("{digest}.bsp"));
		let hashedbz2path = SETTINGS.dir_hashed.join(format!("{digest}.bsp.bz2"));
		let exists = tokio::fs::try_exists(&hashedbz2path).await?;
		let filesize_bz2;

		if !exists {
			/* TODO: timestampFixer
			if timestampFixer:
				print("wtf bad??? {} {}".format(filename, renameto))
				mm.close()
				continue
			*/
			println!("copying new! {} -> {}", entry.display(), hashedbsppath.display());

			#[cfg(target_os="windows")]
			fn fill_filetimes(metadata: &std::fs::Metadata) -> anyhow::Result<std::fs::FileTimes> {
				Ok(std::fs::FileTimes::new().set_modified(metadata.modified()?).set_created(metadata.created()?))
			}
			#[cfg(not(target_os="windows"))]
			fn fill_filetimes(metadata: &std::fs::Metadata) -> anyhow::Result<std::fs::FileTimes> {
				Ok(std::fs::FileTimes::new().set_modified(metadata.modified()?))
			}
			let filetimes = fill_filetimes(&metadata)?;

			let copy_task = tokio::task::spawn_blocking({
				let content = Arc::clone(&content);
				let hashedbsppath = hashedbsppath.clone();
				move || {
					let mut file = std::fs::File::create_new(hashedbsppath)?;
					file.write_all(&content)?;
					file.flush()?;
					file.set_times(filetimes)?;
					std::io::Result::Ok(())
				}
			});

			let bz2_task = tokio::task::spawn_blocking({
				let content = Arc::clone(&content);
				let hashedbz2path = hashedbz2path.clone();
				move || {
					let mut outbuf = std::io::BufWriter::new(std::fs::File::create_new(hashedbz2path)?);
					let mut compressor = bzip2::read::BzEncoder::new(content.as_slice(), bzip2::Compression::best());
					let filesize_bz2 = std::io::copy(&mut compressor, &mut outbuf)?;
					outbuf.flush()?;
					outbuf.into_inner()?.set_times(filetimes)?;
					anyhow::Result::<u64, anyhow::Error>::Ok(filesize_bz2)
				}
			});

			copy_task.await.unwrap()?;
			filesize_bz2 = bz2_task.await.unwrap()?;
		} else {
			// TODO: Read size from processed files instead... Kind of a hassle though.
			/*
			if tokio::fs::metadata(&hashedbsppath).await?.len() != filesize {
				println!("?????? HASH COLLISION WOW {} {}", digest, entry.display());
				continue;
			}
			*/

			let bz2_metadata = tokio::fs::metadata(&hashedbz2path).await?;
			filesize_bz2 = bz2_metadata.len();

			let bz2_modified = bz2_metadata.modified()?;
			let new_modified = metadata.modified()?;

			if new_modified < bz2_modified {
				if timestamp_fixer {
					println!("timestamping {} from {:#?} to {:#?}", digest, bz2_modified, new_modified);
					let times = std::fs::FileTimes::new().set_modified(new_modified);
					std::fs::OpenOptions::new()
						.write(true)
						.open(&hashedbsppath)?
						.set_times(times)?;
					std::fs::OpenOptions::new()
						.write(true)
						.open(&hashedbz2path)?
						.set_times(times)?;
					// TODO: Don't continue?
					continue;
				} else {
					// TODO: fix timestamp format
					println!(
						"older timestamp for {} from {} -- {:#?} -> {:#?}!",
						digest,
						entry.display(),
						bz2_modified,
						new_modified
					);
				}
			}

			if skip_existing_hash {
				continue;
			}
		}

		let mut stem = entry.file_stem().unwrap().to_str().unwrap().to_owned();
		if stem == "#" {
			// stupid
			stem = rand::random::<u64>().to_string();
		}
		if stem.starts_with('#') && stem.len() > 1 {
			stem.remove(0);
		}

		let parent = if mode == Mode::Manual {
			entry.parent().unwrap().strip_prefix(&mapsfolder)?
		} else {
			entry.strip_prefix(mapsfolder.parent().unwrap())?
		};
		let parent = parent.to_str().unwrap().replace('\\', "/");

		if mode == Mode::Automatic {
			let normalized = normalize_mapname(&stem);
			if let Some(existing_names_id) = existing_names.get(&normalized) {
				let in_recents = existing_recents.get(&normalized);
				if in_recents.is_none() || *in_recents.unwrap() != *existing_names_id {
					stem.insert(0, '#');
				}
			}
		} else if canon_clobber_check {
			if let Some(found) = existing_canon.get(&stem) {
				if digest != *found {
					// TODO: (low) canonClobber things...
					// canonClobber.write(f"{steam},{found},\n")
					// print_and_to_shit(f"  ^^^^ name collision {digest} {entry} (existing: {row[0]} & {found}")
				}
			}
		}

		outcsv.write_record([&stem, &filesize.to_string(), &filesize_bz2.to_string(), &digest, &parent])?;

		if !exists {
			newly_hashed.push(UnprocessedCsvRow {
				mapname: stem,
				filesize,
				filesize_bz2,
				sha1: digest,
				note: Some(parent),
				recently_added_note: None,
				datetime: None,
			});
		}
	}

	let mut f = tokio::fs::OpenOptions::new().append(true).open(&outcsvpath).await?;
	f.write_all(&outcsv.into_inner()?).await?;
	f.flush().await?;

	Ok(newly_hashed)
}
