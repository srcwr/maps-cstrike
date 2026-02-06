// SPDX-License-Identifier: WTFPL
// Copyright 2025 rtldg <rtldg@protonmail.com>

/*
- remove name from recently_added.csv
- remove '#' from any gamebanana-x-automatic.csv entries with the same name
- add to canon.csv
*/

use crate::{
	SETTINGS,
	csv::{CanonCsvRow, UnprocessedCsvRow, UnprocessedCsvRowShort},
	normalize_mapname,
};

pub async fn run(name: &str, hash: &str) -> anyhow::Result<()> {
	let name = normalize_mapname(name);
	//let mut recently_added = vec![];

	{
		let mut out_recently_added_csv = csv::Writer::from_writer(vec![]);
		let mut in_recently_added_csv = csv::Reader::from_path(SETTINGS.dir_maps_cstrike.join("recently_added.csv"))?;
		for row in in_recently_added_csv.deserialize::<UnprocessedCsvRow>() {
			let mut row = row?;
			let mapname = row.mapname.trim_start_matches('#');
			let mapname = normalize_mapname(&mapname);
			if mapname == name {
				if row.sha1 != hash {
					println!("removing {row:?}");
					continue;
				}
				if row.mapname.starts_with('#') {
					row.mapname.remove(0);
				}
			}
			out_recently_added_csv.serialize(row)?;
		}
		tokio::fs::write(
			SETTINGS.dir_maps_cstrike.join("recently_added.csv"),
			out_recently_added_csv.into_inner().unwrap(),
		)
		.await?;
	}

	{
		let mut out_gamebanana_x_automatic_csv = csv::Writer::from_writer(vec![]);
		let mut in_gamebanana_x_automatic_csv = csv::Reader::from_path(
			SETTINGS
				.dir_maps_cstrike
				.join("unprocessed")
				.join("gamebanana-x-automatic.csv"),
		)?;
		for row in in_gamebanana_x_automatic_csv.deserialize::<UnprocessedCsvRowShort>() {
			let mut row = row?;
			if let Some(stripped) = row.mapname.strip_prefix('#') {
				let mapname = normalize_mapname(&stripped);
				if mapname == name {
					println!("uncommenting {row:?}");
					row.mapname = stripped.to_string();
				}
			}
			out_gamebanana_x_automatic_csv.serialize(row)?;
		}
		tokio::fs::write(
			SETTINGS
				.dir_maps_cstrike
				.join("unprocessed")
				.join("gamebanana-x-automatic.csv"),
			out_gamebanana_x_automatic_csv.into_inner().unwrap(),
		)
		.await?;
	}

	{
		let mut out_canon_rows = vec![CanonCsvRow {
			mapname: name.to_string(),
			sha1: hash.to_string(),
			note: "ezcanon".to_string(),
		}];
		let mut in_canon_csv = csv::Reader::from_path(SETTINGS.dir_maps_cstrike.join("canon.csv"))?;
		for row in in_canon_csv.deserialize::<CanonCsvRow>() {
			// note: `mapname` is deserialized with `normalize_mapname_column()`
			let row = row?;
			if row.mapname.trim_start_matches('#') == name {
				println!("removing {row:?}");
				continue;
			}
			out_canon_rows.push(row);
		}
		out_canon_rows.sort_by(|a,b| a.mapname.cmp(&b.mapname));
		let mut out_canon_csv = csv::Writer::from_writer(vec![]);
		for row in out_canon_rows {
			out_canon_csv.serialize(row)?;
		}
		tokio::fs::write(
			SETTINGS.dir_maps_cstrike.join("canon.csv"),
			out_canon_csv.into_inner().unwrap(),
		)
		.await?;
	}

	Ok(())
}
