// SPDX-License-Identifier: WTFPL
// Copyright 2025 rtldg <rtldg@protonmail.com>

use std::{collections::BTreeMap, path::Path, sync::Arc};

use serde::{Deserialize, Deserializer, Serialize};

use crate::{SETTINGS, normalize_mapname};

use super::GamebananaID;

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Clone, Debug, PartialOrd, Ord)]
// also for canon.csv
pub struct UnprocessedCsvRow {
	//#[serde(deserialize_with = "normalize_mapname_column")] // dangerous... (fricked with my recently_added.csv...)
	pub mapname: String,
	pub filesize: u64,
	pub filesize_bz2: u64,
	pub sha1: String,
	#[serde(alias = "url")]
	pub note: Option<String>,
	pub recently_added_note: Option<String>,
	pub datetime: Option<compact_str::CompactString>,
}

#[derive(Serialize, Deserialize)]
pub struct ProcessedCsvRow<'a> {
	pub mapname: &'a str,
	pub sha1: &'a str,
	pub filesize: u64,
	pub filesize_bz2: u64,
	pub url: &'a str,
}

pub fn normalize_mapname_column<'de, D>(deserializer: D) -> Result<String, D::Error>
where
	D: Deserializer<'de>,
{
	let buf = String::deserialize(deserializer)?;
	Ok(normalize_mapname(&buf))
}

#[derive(Serialize, Deserialize)]
pub struct ShrimpleRow {
	pub sha1: String,
	pub reason: String,
}

#[derive(Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
pub struct FilelistRow {
	pub filename: String,
	pub size: u64,
	pub compressed: u64,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct CanonCsvRow {
	#[serde(deserialize_with = "normalize_mapname_column")]
	pub mapname: String,
	pub sha1: String,
	pub note: String,
}

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct DownloadsRow {
	pub modid: GamebananaID,
	pub downloadid: GamebananaID,
	pub filename: String,
	#[serde_as(as = "serde_with::BoolFromInt")]
	pub downloaded: bool,
	#[serde_as(as = "serde_with::BoolFromInt")]
	pub processed: bool,
}
pub type Downloads = BTreeMap<(GamebananaID, GamebananaID), DownloadsRow>;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct ModifiedTimesRow {
	pub modid: GamebananaID,
	pub lastmodified: u64,
	#[serde_as(as = "serde_with::BoolFromInt")]
	pub checked: bool,
}
pub type ModifiedTimes = BTreeMap<GamebananaID, ModifiedTimesRow>;

pub(crate) async fn load_downloads() -> anyhow::Result<Downloads> {
	tokio::task::spawn_blocking(|| {
		let mut downloads = BTreeMap::new();
		let mut in_csv = csv::Reader::from_path(SETTINGS.dir_maps_cstrike.join("gamebanana-downloads.csv"))?;
		for row in in_csv.deserialize::<DownloadsRow>() {
			let row = row?;
			assert!(downloads.insert((row.modid, row.downloadid), row).is_none());
		}
		Ok(downloads)
	})
	.await?
}

pub(crate) async fn write_downloads(downloads: Arc<Downloads>) -> anyhow::Result<()> {
	tokio::task::spawn_blocking(move || {
		let mut out_csv = csv::Writer::from_path(SETTINGS.dir_maps_cstrike.join("gamebanana-downloads.csv"))?;
		for ((_modid, _downloadid), row) in downloads.iter() {
			out_csv.serialize(row)?;
		}
		out_csv.flush()?;
		Ok(())
	})
	.await?
}

pub(crate) async fn load_modified_times() -> anyhow::Result<ModifiedTimes> {
	tokio::task::spawn_blocking(|| {
		let mut downloads = BTreeMap::new();
		let mut in_csv = csv::Reader::from_path(SETTINGS.dir_maps_cstrike.join("gamebanana-modified-times.csv"))?;
		for row in in_csv.deserialize::<ModifiedTimesRow>() {
			let row = row?;
			assert!(downloads.insert(row.modid, row).is_none());
		}
		Ok(downloads)
	})
	.await?
}

pub(crate) async fn write_modified_times(modified_times: Arc<ModifiedTimes>) -> anyhow::Result<()> {
	tokio::task::spawn_blocking(move || {
		let mut out_csv = csv::Writer::from_path(SETTINGS.dir_maps_cstrike.join("gamebanana-modified-times.csv"))?;
		for (_modid, row) in modified_times.iter() {
			out_csv.serialize(row)?;
		}
		out_csv.flush()?;
		Ok(())
	})
	.await?
}

/// Assumes that the "filename" column will not cause CSV problems....
pub(crate) async fn fill_downloads<P: AsRef<Path>>(srcdir: P) -> anyhow::Result<()> {
	let srcdir = srcdir.as_ref();
	let outcsv = SETTINGS.dir_maps_cstrike.join("gamebanana-downloads.csv");

	let mut records = vec![];
	let mut dir = tokio::fs::read_dir(srcdir).await?;
	while let Some(entry) = dir.next_entry().await? {
		if !entry.file_type().await?.is_file() {
			continue;
		}
		let filename = entry.file_name().to_str().unwrap().to_owned();
		let splits: Vec<&str> = filename.splitn(3, '_').collect();
		records.push(DownloadsRow {
			modid: splits[0].parse()?,
			downloadid: splits[1].parse()?,
			filename: splits[2].to_owned(),
			downloaded: true,
			processed: true,
		});
	}
	records.sort();
	tokio::task::spawn_blocking(move || {
		let mut out_csv = csv::Writer::from_path(outcsv)?;
		for record in records {
			out_csv.serialize(record)?;
		}
		out_csv.flush()?;
		Ok(())
	})
	.await?
}
