// SPDX-License-Identifier: WTFPL

#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};

pub type GamebananaID = u64;

/*
#[derive(Serialize, Deserialize)]
pub struct AMetadata1 {
	pub _bIsComplete: bool,
	pub _nPerpage: usize,
	pub _nRecordCount: usize,
}
*/

#[derive(Serialize, Deserialize)]
pub struct GbSubmitter {
	pub _idRow: GamebananaID,
	pub _sAvatarUrl: String,
	pub _sName: String,
	pub _sProfileUrl: String,
}

// na=N/A, no=None, mi=Minor, mo=Moderate, ma=Major
#[derive(Serialize, Deserialize)]
pub struct GbAiUsage {
	art: String,
	code: String,
	audio: String,
	concept: String,
	profile: String,
}

#[derive(Serialize, Deserialize)]
pub struct GbRecords {
	//pub _aSubmitter: ASubmitter1,
	//pub _bHasFiles: bool,
	pub _idRow: GamebananaID,
	//pub _sName: String,
	//_tsDateAdded: u64,
	pub _tsDateModified: u64,
	//_tsDateUpdated: u64,
}
#[derive(Serialize, Deserialize)]
pub struct GbApiV13ModIndex {
	//pub _aMetadata: AMetadata1,
	pub _aRecords: Vec<GbRecords>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct GbFiles {
	//_bContainsExe: bool,
	pub _idRow: GamebananaID,
	//_nDownloadCount: u64,
	//_nFilesize: u64,
	//_sAnalysisResult: String,
	//pub _sAnalysisResultCode: String,
	//pub _sAnalysisState: String,
	//_sAvastAvResult: Option<String>,
	//_sClamAvResult: Option<String>,
	//_sDescription: String,
	//pub _sDownloadUrl: String,
	pub _sFile: String,
	//pub _sMd5Checksum: String, // can be an empty string
	//_tsDateAdded: u64,
}

#[derive(Serialize, Deserialize)]
pub(super) struct GbApiV13Mod {
	pub _aFiles: Option<Vec<GbFiles>>,
	pub _aAiUsage: Option<GbAiUsage>,
	pub _aSubmitter: GbSubmitter,
}

#[derive(Deserialize)]
pub(super) struct KsfMap {
	pub name: String,
	pub link: Option<String>,
}
