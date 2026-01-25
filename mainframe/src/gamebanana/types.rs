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
#[derive(Serialize, Deserialize)]
pub struct ASubmitter1 {
	pub _idRow: GamebananaID,
	pub _sAvatarUrl: String,
	pub _sName: String,
	pub _sProfileUrl: String,
}
*/
#[derive(Serialize, Deserialize)]
pub struct ARecords1 {
	//pub _aSubmitter: ASubmitter1,
	//pub _bHasFiles: bool,
	pub _idRow: GamebananaID,
	//pub _sName: String,
	//_tsDateAdded: u64,
	pub _tsDateModified: u64,
	//_tsDateUpdated: u64,
}
#[derive(Serialize, Deserialize)]
pub struct ApiV11ModIndex {
	//pub _aMetadata: AMetadata1,
	pub _aRecords: Vec<ARecords1>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct AFiles1 {
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
pub(super) struct ApiV11Mod {
	pub _aFiles: Option<Vec<AFiles1>>,
}
