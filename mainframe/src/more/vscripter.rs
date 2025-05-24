// SPDX-License-Identifier: WTFPL
// Copyright 2025 rtldg <rtldg@protonmail.com>

use std::{collections::BTreeMap, sync::LazyLock};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use crate::{Bsps, SETTINGS, hash_to_hex};

#[derive(Serialize, Deserialize, Default)]
struct Row {
	sha1: String,
	reason: String,
}

pub(crate) fn in_entities(entities: &[u8]) -> bool {
	// bytes::Regex for this because this shit is stupid as fuck and Entities Lumps use random ANSI code pages...
	static RE_ENTITIES: LazyLock<regex::bytes::Regex> = LazyLock::new(|| {
		regex::bytes::RegexBuilder::new("(script |script_execute|RunScriptCode|RunScriptFile|CallScriptFunction)")
			.case_insensitive(true)
			.build()
			.unwrap()
	});
	RE_ENTITIES.is_match(entities)
}

pub(crate) fn in_filelist(filelist: &[u8]) -> bool {
	// bytes::Regex just for convenience
	static RE_FILELIST: LazyLock<regex::bytes::Regex> = LazyLock::new(|| {
		regex::bytes::RegexBuilder::new("(vscripts|\\.nut)")
			.case_insensitive(true)
			.build()
			.unwrap()
	});
	RE_FILELIST.is_match(filelist)
}

pub(crate) fn insert_into_csv(vscripts: &[(String, bool, bool)]) -> anyhow::Result<()> {
	if vscripts.is_empty() {
		return Ok(());
	}

	let csv_path = SETTINGS.dir_maps_cstrike_more.join("vscript_probably.csv");

	let mut in_csv = csv::Reader::from_path(&csv_path)?;
	let mut contents = BTreeMap::<String, String>::new();
	for row in in_csv.deserialize::<Row>() {
		let row = row?;
		let _ = contents.insert(row.sha1, row.reason);
	}
	drop(in_csv);

	for (hash, ents, files) in vscripts {
		let _ = contents.insert(
			hash.clone(),
			if *ents && *files {
				"filelist & entities"
			} else if *ents {
				"entities"
			} else {
				"filelist"
			}
			.to_string(),
		);
	}

	let mut out_csv = csv::Writer::from_path(&csv_path)?;
	for (sha1, reason) in contents {
		out_csv.serialize(Row { sha1, reason })?;
	}

	out_csv.flush()?;
	Ok(())
}

pub(crate) fn run(hashes: &Bsps) -> anyhow::Result<()> {
	let vscripts = hashes
		.par_iter()
		.filter_map(|hash| {
			let hash = hash_to_hex(hash);
			let ents = std::fs::read(SETTINGS.dir_maps_cstrike_more.join(format!("entities/{hash}.cfg")))
				.map(|bytes| in_entities(&bytes))
				.ok()
				.unwrap_or(false);
			let files = std::fs::read(SETTINGS.dir_maps_cstrike_more.join(format!("filelist/{hash}.csv")))
				.map(|bytes| in_filelist(&bytes))
				.ok()
				.unwrap_or(false);
			if ents || files { Some((hash, ents, files)) } else { None }
		})
		.collect::<Vec<_>>();

	insert_into_csv(&vscripts)?;

	Ok(())
}
