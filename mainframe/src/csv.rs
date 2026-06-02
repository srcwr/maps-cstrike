// SPDX-License-Identifier: WTFPL
// Copyright 2025 rtldg <rtldg@protonmail.com>

use std::{
	collections::BTreeMap,
	path::{Path, PathBuf},
	sync::Arc,
};

use anyhow::Context;
use itertools::Itertools;
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

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Clone, Debug, PartialOrd, Ord)]
// also for canon.csv
pub struct UnprocessedCsvRowShort {
	//#[serde(deserialize_with = "normalize_mapname_column")] // dangerous... (fricked with my recently_added.csv...)
	pub mapname: String,
	pub filesize: u64,
	pub filesize_bz2: u64,
	pub sha1: String,
	#[serde(alias = "url")]
	pub note: Option<String>,
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

#[derive(Deserialize)]
pub struct CanonGbCSsvRow {
	pub sha1: String,
	pub gbnote: String,
	#[allow(unused)]
	pub mynote: String,
}

#[derive(Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
pub struct FilelistRow {
	pub filename: String,
	pub size: u64,
	pub compressed: u64,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, PartialOrd, Ord, Debug)]
pub struct CanonCsvRow {
	#[serde(deserialize_with = "normalize_mapname_column")]
	pub mapname: String,
	pub sha1: String,
	pub note: String,
}

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct GbDownloadsRow {
	pub modid: GamebananaID,
	pub downloadid: GamebananaID,
	pub filename: String,
	#[serde_as(as = "serde_with::BoolFromInt")]
	pub downloaded: bool,
	#[serde_as(as = "serde_with::BoolFromInt")]
	pub processed: bool,
}
pub type GbDownloads = BTreeMap<(GamebananaID, GamebananaID), GbDownloadsRow>;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct GbModifiedTimesRow {
	pub modid: GamebananaID,
	pub lastmodified: u64,
	#[serde_as(as = "serde_with::BoolFromInt")]
	pub checked: bool,
}
pub type ModifiedTimes = BTreeMap<GamebananaID, GbModifiedTimesRow>;

// what the fuck am I doing, man...
pub(crate) async fn load_csv<Key, Row, RowToKey>(path: PathBuf, row_to_key: RowToKey) -> anyhow::Result<BTreeMap<Key, Row>>
where
	Key: Ord + Send + Sync + 'static,
	Row: serde::de::DeserializeOwned + Send + 'static,
	RowToKey: Fn(&Row) -> Key + Send + Sync + 'static,
{
	tokio::task::spawn_blocking(move || {
		let mut out = BTreeMap::new();
		let mut in_csv = csv::Reader::from_path(path)?;
		for row in in_csv.deserialize::<Row>() {
			let row = row?;
			assert!(out.insert(row_to_key(&row), row).is_none());
		}
		Ok(out)
	})
	.await?
}

pub(crate) async fn write_csv<Key, Row>(path: PathBuf, data: Arc<BTreeMap<Key, Row>>) -> anyhow::Result<()>
where
	Key: Ord + Send + Sync + 'static,
	Row: serde::ser::Serialize + Send + Sync + 'static,
{
	tokio::task::spawn_blocking(move || {
		let mut out_csv = csv::Writer::from_path(path)?;
		for (_key, row) in data.iter() {
			out_csv.serialize(row)?;
		}
		out_csv.flush()?;
		Ok(())
	})
	.await?
}

pub(crate) async fn load_gbdownloads() -> anyhow::Result<GbDownloads> {
	/*
	if true {
		return load_csv("downloader-state/gamebanana-downloads.csv", |row: &GbDownloadsRow| {
			(row.modid, row.downloadid)
		})
		.await;
	}
	*/

	tokio::task::spawn_blocking(|| {
		let mut downloads = BTreeMap::new();
		let mut in_csv = csv::Reader::from_path(SETTINGS.dir_maps_cstrike.join("downloader-state/gamebanana-downloads.csv"))?;
		for row in in_csv.deserialize::<GbDownloadsRow>() {
			let row = row?;
			assert!(downloads.insert((row.modid, row.downloadid), row).is_none());
		}
		Ok(downloads)
	})
	.await?
}

pub(crate) async fn write_gbdownloads(downloads: Arc<GbDownloads>) -> anyhow::Result<()> {
	tokio::task::spawn_blocking(move || {
		let mut out_csv = csv::Writer::from_path(SETTINGS.dir_maps_cstrike.join("downloader-state/gamebanana-downloads.csv"))?;
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
		let mut in_csv = csv::Reader::from_path(
			SETTINGS
				.dir_maps_cstrike
				.join("downloader-state/gamebanana-modified-times.csv"),
		)?;
		for row in in_csv.deserialize::<GbModifiedTimesRow>() {
			let row = row?;
			assert!(downloads.insert(row.modid, row).is_none());
		}
		Ok(downloads)
	})
	.await?
}

pub(crate) async fn write_modified_times(modified_times: Arc<ModifiedTimes>) -> anyhow::Result<()> {
	tokio::task::spawn_blocking(move || {
		let mut out_csv = csv::Writer::from_path(
			SETTINGS
				.dir_maps_cstrike
				.join("downloader-state/gamebanana-modified-times.csv"),
		)?;
		for (_modid, row) in modified_times.iter() {
			out_csv.serialize(row)?;
		}
		out_csv.flush()?;
		Ok(())
	})
	.await?
}

pub(crate) async fn fill_downloads<P: AsRef<Path>>(srcdir: P) -> anyhow::Result<()> {
	let srcdir = srcdir.as_ref();
	let outcsv = SETTINGS.dir_maps_cstrike.join("downloader-state/gamebanana-downloads.csv");

	let mut records = vec![];
	let mut dir = tokio::fs::read_dir(srcdir).await?;
	while let Some(entry) = dir.next_entry().await? {
		if !entry.file_type().await?.is_file() {
			continue;
		}
		let filename = entry.file_name().to_str().unwrap().to_owned();
		let (modid, downloadid, filename) = filename
			.splitn(3, '_')
			.collect_tuple()
			.context("shouldn't have issues splitting here, but alas...")?;
		records.push(GbDownloadsRow {
			modid: modid.parse()?,
			downloadid: downloadid.parse()?,
			filename: filename.to_owned(),
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
