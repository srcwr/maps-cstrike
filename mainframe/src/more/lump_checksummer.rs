// SPDX-License-Identifier: WTFPL
// Copyright 2025 rtldg <rtldg@protonmail.com>

use std::collections::BTreeMap;

use md5::{Digest, Md5};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use crate::{BspHash, Bsps, SETTINGS, csv::ShrimpleRow, hash_to_hex, hex_to_hash};

#[derive(Serialize, Deserialize, Default)]
struct LumpChecksumsRow {
	sha1: String,
	lump_md5_checksum: String,
}

fn get_checksum(hash: &BspHash) -> anyhow::Result<String> {
	let content = std::fs::read(SETTINGS.dir_hashed.join(format!("{}.bsp", hash_to_hex(hash))))?;
	let bspfile = vbsp::bspfile::BspFile::new(&content)?;

	let mut hasher = Md5::new();

	// start at 1 to skip entities-lump (which is 0)
	for lump_type in 1u32..=63 {
		let raw_lump = bspfile.get_lump_raw(bspfile.get_lump_entry(lump_type.try_into().unwrap()))?;
		hasher.update(raw_lump);
	}

	let digest = hasher.finalize();
	Ok(const_hex::encode(digest))
}

pub(crate) fn run(bsps: &Bsps) -> anyhow::Result<()> {
	let mut ignore_csv = csv::Reader::from_path(SETTINGS.dir_maps_cstrike_more.join("ignore.csv"))?;
	let mut ignored = Bsps::new();
	for row in ignore_csv.deserialize::<ShrimpleRow>() {
		let row = row?;
		let _ = ignored.insert(hex_to_hash(row.sha1));
	}

	let csv_path = SETTINGS.dir_maps_cstrike_more.join("lump_checksums.csv");
	let mut in_csv = csv::Reader::from_path(&csv_path)?;
	let mut contents = BTreeMap::<String, String>::new();
	for row in in_csv.deserialize::<LumpChecksumsRow>() {
		let row = row?;
		ignored.insert(hex_to_hash(&row.sha1));
		let _ = contents.insert(row.sha1, row.lump_md5_checksum);
	}
	drop(in_csv);

	let checksums = bsps
		.difference(&ignored)
		.cloned()
		.collect::<Vec<_>>()
		.par_iter()
		.filter_map(|hash| Some((*hash, get_checksum(hash).ok()?)))
		.collect::<Vec<_>>();

	for (hash, checksum) in checksums {
		//println!("{} {}", hash_to_hex(&hash), checksum);
		let _ = contents.insert(hash_to_hex(&hash), checksum);
	}

	let mut out_csv = csv::Writer::from_path(&csv_path)?;
	for (sha1, lump_md5_checksum) in contents {
		out_csv.serialize(LumpChecksumsRow { sha1, lump_md5_checksum })?;
	}

	out_csv.flush()?;

	Ok(())
}
