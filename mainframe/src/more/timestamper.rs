// SPDX-License-Identifier: WTFPL
// Copyright 2025 rtldg <rtldg@protonmail.com>

use std::{
	cmp::Ordering,
	collections::{BTreeMap, HashMap},
};

use jiff::{SpanRelativeTo, Timestamp, ToSpan, civil::DateTime, tz::TimeZone};
use serde::{Deserialize, Serialize};

use crate::{BspHash, Bsps, SETTINGS, hash_to_hex, hex_to_hash};

#[derive(Serialize, Deserialize, Default)]
struct Row {
	sha1: String,
	r#mod: String,
	create: String,
}

fn ymdhms(t: Timestamp) -> String {
	// fuck subseconds after all...
	t.strftime("%Y-%m-%d %H:%M:%S").to_string()
}

pub(crate) fn run(bsps: Option<&Bsps>) -> anyhow::Result<()> {
	let csv_path = SETTINGS.dir_maps_cstrike_more.join("timestamps.csv");

	let mut in_csv = csv::Reader::from_path(&csv_path)?;
	let mut timestamps = BTreeMap::<BspHash, (String, String)>::new();
	for row in in_csv.deserialize::<Row>() {
		let row = row?;
		let _ = timestamps.insert(hex_to_hash(&row.sha1), (row.r#mod, row.create));
	}
	drop(in_csv);

	let mut new_maps: HashMap<BspHash, (Timestamp, Timestamp)> = HashMap::new();

	if let Some(bsps) = bsps {
		for bsp in bsps {
			let filename = SETTINGS.dir_hashed.join(format!("{}.bsp.bz2", hash_to_hex(bsp)));
			let metadata = std::fs::metadata(&filename)?;
			let _ = new_maps.insert(*bsp, (metadata.modified()?.try_into()?, metadata.created()?.try_into()?));
		}
	} else {
		for entry in std::fs::read_dir(&SETTINGS.dir_hashed)? {
			let entry = entry?;
			if let Some(ext) = entry.path().extension() {
				if ext.eq_ignore_ascii_case("bz2") {
					let metadata = entry.metadata()?;
					let _ = new_maps.insert(
						hex_to_hash(entry.path().file_stem().unwrap().to_str().unwrap().trim_end_matches(".bsp")),
						(metadata.modified()?.try_into()?, metadata.created()?.try_into()?),
					);
				}
			}
		}
	}

	for (hash, (new_mod, new_create)) in new_maps {
		if let Some((orig_mod, orig_create)) = timestamps.get(&hash) {
			// dogshit to stop clobbering tens-of-thousands of rows because jiff & python format microseconds differently
			if (new_mod.to_zoned(TimeZone::UTC).datetime() - orig_mod.parse::<DateTime>()?)
				.abs()
				.compare((2.seconds(), SpanRelativeTo::days_are_24_hours()))?
				== Ordering::Less
			{
				continue;
			}
			// moving files to different drives blew up my creation timestamps :^) TODO
			if (new_create.to_zoned(TimeZone::UTC).datetime() - orig_create.parse::<DateTime>()?)
				.abs()
				.compare((2.seconds(), SpanRelativeTo::days_are_24_hours()))?
				== Ordering::Greater
			{
				continue;
			}
		}
		let _ = timestamps.insert(hash, (ymdhms(new_mod), ymdhms(new_create)));
	}

	//dbg!(timestamps.len());

	let mut out_csv = csv::Writer::from_path(&csv_path)?;
	for (hash, (r#mod, create)) in timestamps {
		out_csv.serialize(Row {
			sha1: hash_to_hex(&hash),
			r#mod,
			create,
		})?;
	}

	out_csv.flush()?;
	Ok(())
}
