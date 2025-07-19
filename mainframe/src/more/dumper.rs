// SPDX-License-Identifier: WTFPL
// Copyright 2025 rtldg <rtldg@protonmail.com>

use std::{collections::HashSet, io::Write, sync::Arc};

use flate2::write::GzEncoder;
use vbsp::bspfile::LumpType;

use crate::{
	Bsps, SETTINGS,
	csv::{FilelistRow, ShrimpleRow},
	hex_to_hash, more,
};

fn get_filelist(data: &[u8]) -> anyhow::Result<Vec<FilelistRow>> {
	let mut rows = vec![];
	let mut zip = zip::ZipArchive::new(std::io::Cursor::new(data))?;
	for i in 0..zip.len() {
		let file = zip.by_index_raw(i)?;
		rows.push(FilelistRow {
			filename: file.name().to_owned(),
			size: file.size(),
			compressed: file.compressed_size(),
		});
	}
	rows.sort();
	Ok(rows)
}

pub(crate) async fn dumpher() -> anyhow::Result<Bsps> {
	let ignored = tokio::task::spawn_blocking(|| {
		let mut in_csv = csv::Reader::from_path(SETTINGS.dir_maps_cstrike_more.join("ignore.csv"))?;
		let mut data = HashSet::new();
		for row in in_csv.deserialize::<ShrimpleRow>() {
			let row = row?;
			data.insert(hex_to_hash(&row.sha1));
		}
		anyhow::Result::<_, anyhow::Error>::Ok(data)
	});
	let ignored_pak = tokio::task::spawn_blocking(|| {
		let mut in_csv = csv::Reader::from_path(SETTINGS.dir_maps_cstrike_more.join("ignore_pak.csv"))?;
		let mut data = HashSet::new();
		for row in in_csv.deserialize::<ShrimpleRow>() {
			let row = row?;
			data.insert(hex_to_hash(&row.sha1));
		}
		anyhow::Result::<_, anyhow::Error>::Ok(data)
	});

	let globbed_bsps = tokio::task::spawn_blocking(|| {
		let mut hashes = HashSet::new();
		for entry in glob::glob(SETTINGS.dir_hashed.join("*.bsp").to_str().unwrap())? {
			let entry = entry?;
			hashes.insert(entry.file_stem().unwrap().to_str().unwrap().to_owned());
		}
		anyhow::Result::<_, anyhow::Error>::Ok(hashes)
	});
	let existing_ents = tokio::task::spawn_blocking(|| {
		let mut hashes = HashSet::new();
		for entry in glob::glob(SETTINGS.dir_maps_cstrike_more.join("entities/*.cfg").to_str().unwrap())? {
			let entry = entry?;
			hashes.insert(entry.file_stem().unwrap().to_str().unwrap().to_owned());
		}
		anyhow::Result::<_, anyhow::Error>::Ok(hashes)
	});
	let existing_filelist = tokio::task::spawn_blocking(|| {
		let mut hashes = HashSet::new();
		for entry in glob::glob(SETTINGS.dir_maps_cstrike_more.join("filelist/*.csv").to_str().unwrap())? {
			let entry = entry?;
			hashes.insert(entry.file_stem().unwrap().to_str().unwrap().to_owned());
		}
		anyhow::Result::<_, anyhow::Error>::Ok(hashes)
	});

	let ignored = ignored.await??;
	let ignored_pak = ignored_pak.await??;
	let globbed_bsps = globbed_bsps.await??;
	let existing_entities = existing_ents.await??;
	let existing_filelist = existing_filelist.await??;

	//let mut vscript_entities = HashSet::new();
	//let mut vscript_filelist = HashSet::new();

	let mut ignore_csv = csv::Writer::from_writer(
		std::fs::OpenOptions::new()
			.append(true)
			.open(SETTINGS.dir_maps_cstrike_more.join("ignore.csv"))?,
	);
	let mut ignore_pak_csv = csv::Writer::from_writer(
		std::fs::OpenOptions::new()
			.append(true)
			.open(SETTINGS.dir_maps_cstrike_more.join("ignore_pak.csv"))?,
	);

	let mut vscripts = vec![];
	let mut bsps = Bsps::new();

	for hash in globbed_bsps {
		let smol_hash = hex_to_hash(&hash);
		if ignored.contains(&smol_hash) || ignored_pak.contains(&smol_hash) {
			continue;
		}
		if existing_entities.contains(&hash) && existing_filelist.contains(&hash) {
			continue;
		}

		// TODO: rework to read & unzip bz2s...
		let content = Arc::new(tokio::fs::read(&SETTINGS.dir_hashed.join(format!("{hash}.bsp"))).await?);
		let bsp = match vbsp::bspfile::BspFile::new(&content) {
			Err(e) => {
				println!("bspfile failed on {hash}\n{e:?}");
				ignore_csv.write_record([&hash, "failed to parse bsp"])?;
				continue;
			}
			Ok(bsp) => bsp,
		};

		bsps.insert(smol_hash);

		let mut in_entities = false;
		let mut in_filelist = false;

		if !existing_entities.contains(&hash) && !ignored.contains(&smol_hash) {
			match bsp.get_lump(bsp.get_lump_entry(LumpType::Entities)) {
				Err(e) => {
					println!("failed to get entities lump on {hash}\n{e:?}");
					ignore_csv.write_record([&hash, &format!("entities {e}")])?;
				}
				Ok(entities) => {
					// random check for corruption
					let mut entities = &entities[..];
					if let Some(eof) = entities.iter().position(|x| *x == b'\0') {
						entities = &entities[..eof];
					}
					if entities.is_empty() {
						println!("entities lumps is empty or corrupt? {hash}");
						ignore_csv.write_record([&hash, "entities lumps is empty or corrupt?"])?;
					} else {
						tokio::fs::write(SETTINGS.dir_maps_cstrike_more.join(format!("entities/{hash}.cfg")), &entities).await?;
						// should spawn_block() here but it just complicates it slightly so who cares...
						let mut encoder = GzEncoder::new(vec![], flate2::Compression::best());
						encoder.write_all(entities)?;
						tokio::fs::write(
							SETTINGS.dir_maps_cstrike_more.join(format!("entitiesgz/{hash}.cfg.gz")),
							encoder.finish()?,
						)
						.await?;
						println!("wrote entities/{hash}.cfg.gz");
						in_entities = more::vscripter::in_entities(entities);
					}
				}
			}
		}

		if !existing_filelist.contains(&hash) && !ignored_pak.contains(&smol_hash) {
			match bsp.get_lump(bsp.get_lump_entry(LumpType::PakFile)) {
				Err(e) => {
					println!("failed to get pakfile lump on {hash}\n{e:?}");
					ignore_pak_csv.write_record([&hash, &format!("{e}")])?;
				}
				Ok(lump) => match get_filelist(&lump) {
					Err(e) => {
						println!("failed to read pakfile zip on {hash}\n{e:?}");
						ignore_pak_csv.write_record([&hash, &format!("{e}")])?;
					}
					Ok(filelist) => {
						let mut out_csv = csv::Writer::from_writer(vec![]);
						if filelist.is_empty() {
							out_csv.write_record(["filename", "size", "compressed"])?;
						} else {
							for row in filelist {
								out_csv.serialize(row)?;
							}
						}
						let filelist = out_csv.into_inner()?;
						tokio::fs::write(SETTINGS.dir_maps_cstrike_more.join(format!("filelist/{hash}.csv")), &filelist).await?;
						println!("wrote filelist/{hash}.csv");
						in_filelist = more::vscripter::in_filelist(&filelist);
					}
				},
			}
		}

		if in_entities || in_filelist {
			vscripts.push((hash.clone(), in_entities, in_filelist));
		}
	}

	ignore_csv.flush()?;
	ignore_pak_csv.flush()?;

	more::vscripter::insert_into_csv(&vscripts)?;

	Ok(bsps)
}
