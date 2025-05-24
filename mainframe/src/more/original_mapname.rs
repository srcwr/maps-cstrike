// SPDX-License-Identifier: WTFPL
// Copyright 2025 rtldg <rtldg@protonmail.com>

use std::{
	collections::BTreeMap,
	path::{Path, PathBuf},
};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use crate::{Bsps, SETTINGS, hash_to_hex};

#[derive(Serialize, Deserialize, Default)]
struct FilelistRow {
	filename: PathBuf,
	size: u64,
	compressed: u64,
}

#[derive(Serialize, Deserialize, Default)]
struct OriginalMapnameRow {
	sha1: String,
	mapname: String,
}

fn get_original_name(csv_path: &Path) -> Option<String> {
	let mut in_csv = csv::Reader::from_path(csv_path).ok()?;
	let mut ain = None;
	let mut any_case_matmaps = None;
	for row in in_csv.deserialize::<FilelistRow>() {
		let row = row.unwrap();
		//dbg!(&row.filename);
		if row.filename.file_name().is_none() {
			println!("{} has a stupid empty filename wtf", csv_path.display());
			continue;
		}
		if row.filename.file_name().unwrap().eq_ignore_ascii_case("cubemapdefault.vtf") {
			if let Some(parent) = row.filename.parent() {
				return Some(parent.file_name().unwrap().to_str().unwrap().to_string());
			}
		}
		if let Some(ext) = row.filename.extension() {
			if ext.eq_ignore_ascii_case("ain") {
				ain = Some(row.filename.file_stem().unwrap().to_str().unwrap().to_owned());
			}
		}
		if row.filename.starts_with("maps/") {
			let stem = Path::new(row.filename.iter().nth(1).unwrap())
				.file_stem()
				.unwrap()
				.to_str()
				.unwrap();

			if stem != "graphs" && stem != "soundcache" && stem != "particles_manifest" && !stem.ends_with("particles") {
				return Some(stem.to_owned());
			}
		}
		if row.filename.starts_with("materials/maps/") {
			// specifically because of de_xmas_hotel...
			return Some(row.filename.iter().nth(2).unwrap().to_str().unwrap().to_owned());
		}
		if any_case_matmaps.is_none() {
			let mut components = row.filename.iter();
			if let Some(a) = components.next() {
				if a.eq_ignore_ascii_case("materials") {
					if let Some(b) = components.next() {
						if b.eq_ignore_ascii_case("maps") {
							any_case_matmaps = components.next().map(|s| s.to_str().unwrap().to_owned());
						}
					}
				}
			}
		}
	}
	ain.or(any_case_matmaps)
}

pub(crate) fn run(hashes: &Bsps) -> anyhow::Result<()> {
	let names = hashes
		.par_iter()
		.filter_map(|hash| {
			let sha1 = hash_to_hex(hash);
			let original_name = get_original_name(&SETTINGS.dir_maps_cstrike_more.join(format!("filelist/{sha1}.csv")))?;
			Some((sha1, original_name))
		})
		.collect::<Vec<_>>();

	if names.is_empty() {
		return Ok(());
	}

	let csv_path = SETTINGS.dir_maps_cstrike_more.join("original_mapname.csv");
	let mut in_csv = csv::Reader::from_path(&csv_path)?;
	let mut contents = BTreeMap::<String, String>::new();
	for row in in_csv.deserialize::<OriginalMapnameRow>() {
		let row = row?;
		let _ = contents.insert(row.sha1, row.mapname);
	}
	drop(in_csv);

	for (sha1, original_name) in names {
		let _ = contents.insert(sha1, original_name);
	}

	let mut out_csv = csv::Writer::from_path(&csv_path)?;
	for (sha1, mapname) in contents {
		out_csv.serialize(OriginalMapnameRow { sha1, mapname })?;
	}

	out_csv.flush()?;
	Ok(())
}
