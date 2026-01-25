// SPDX-License-Identifier: WTFPL
// Copyright 2022, 2025 rtldg <rtldg@protonmail.com>

mod base; // maps-cstrike
mod cloudflare;
mod csv;
mod discord;
mod gamebanana;
mod more; // maps-cstrike-more

#[global_allocator]
static GLOBAL_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

type BspHash = [u8; 20];
type Bsps = BTreeSet<BspHash>;
fn hex_to_hash<S: AsRef<str>>(s: S) -> BspHash {
	const_hex::decode_to_array(s.as_ref()).unwrap()
}
fn hash_to_hex(h: &BspHash) -> String {
	const_hex::encode(h)
}

fn normalize_mapname(s: &str) -> String {
	s.trim().to_ascii_lowercase().replace('.', "_")
}

// TODO: function to sort by mapname that matches

use std::{
	collections::{BTreeSet, HashMap},
	num::NonZeroUsize,
	path::{Path, PathBuf},
	sync::LazyLock,
	time::Duration,
};

use clap::{Parser, Subcommand};
use gamebanana::GamebananaID;
use serde::Deserialize;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None, flatten_help = true, disable_help_subcommand=true)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
	// bleh
	SemiManual {
		timestamp: i64,
	},
	ProcessAndTransfer,
	// maps-cstrike
	Mapshasher {
		#[arg(short, long)]
		timestamp_fixer: bool,
		#[arg(short, long)]
		skip_existing_hash: bool,
		#[arg(short, long)]
		canon_clobber_check: bool,
	},
	Autogb,
	Process,
	Gbdls,
	Cfpages,
	TransferToNodes,
	Ezcanon {
		name: String,
		hash: String,
	},
	// maps-cstrike-more
	Auto2 {
		timestamp: i64,
	},
	Dumper,
	LumpChecksummer,
	OriginalFilename,
	Timestamper,
	Venus,
	Vscripter,
	// cloudflare
	PurgeCache,
	SyncR2Bz2s,
}

#[derive(Deserialize)]
pub struct BucketSettings {
	pub name: String,
	pub s3_access_key_id: String,
	pub s3_access_key_secret: String,
}

#[derive(Deserialize)]
pub struct GlobalSettings {
	/// the committer's name to use for commits
	pub git_name: String,
	/// the committer's email to use for commits
	pub git_email: String,
	/// the remote to push to
	pub git_origin: String,

	/// the zone id on cloudflare
	pub cf_zone: String,
	/// a token that has `Cache Purge:Purge` permissions on your zone
	pub cf_purgetoken: String,
	/// fuck documenting
	pub buckets: HashMap<String, BucketSettings>,
	/// the account id used in r2 (which is the same as the cloudflare "account" id)
	pub r2_account_id: String,

	/// the discord webhook to post new downloads to
	pub discord_webhook: String,
	/// a ping identifier. e.g. "<@&123123123123>" to ping a role
	pub discord_ping: String,
	/// the discord username used for messages posted via webhook
	pub discord_username: String,
	/// the bot token used for member bots...
	pub discord_bottoken: String,

	/// the full path to the maps-cstrike repo on disk
	pub dir_maps_cstrike: PathBuf,
	/// the full path to the maps-cstrike-more repo on disk
	pub dir_maps_cstrike_more: PathBuf,
	/// the full path to the folder you store the hashed .bsp and .bsp.bz2 files
	pub dir_hashed: PathBuf,
	/// the full path to the folder you store gamebanana downloads in
	pub dir_gamebanana_scrape: PathBuf,
	/// the full path to the folder that gamebanana downloads are extracted to
	pub dir_gamebanana_auto: PathBuf,
	/// the full path to the folder you manually put .bsp files into for maps-hasher & process & etc...
	pub dir_manualmaps: PathBuf,

	/// commands to use to transfer to nodes
	pub cmd_transfer_to_nodes: Vec<Vec<Vec<String>>>,

	/// gamebanana category ids
	pub gb_categories: Vec<GamebananaID>,
	/// Max of 50.
	pub gb_maxperpage: NonZeroUsize,
	///
	pub gb_perpage: NonZeroUsize,
	///
	pub gb_numtofetch: NonZeroUsize,
	///
	pub gb_itemoffset: usize,
	/// Added because caching is ruining my life...
	pub gb_walkto: usize,

	///
	pub gb_wait_regular: f32,
	///
	pub gb_wait_looped: f32,
}

// TODO: (low) use a .env file or something...
static SETTINGS: LazyLock<GlobalSettings> = LazyLock::new(|| {
	let mut dir = std::env::current_dir().unwrap();
	loop {
		if let Ok(content) = std::fs::read_to_string(dir.join("secrets.json")) {
			return serde_json::from_str(content.trim()).unwrap();
		}
		if !dir.pop() {
			panic!("failed to find secrets.json");
		}
	}
});

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
	reqwest::ClientBuilder::new()
		.connect_timeout(Duration::from_secs(10))
		.read_timeout(Duration::from_secs(120))
		.timeout(Duration::from_secs(140))
		.user_agent(format!(
			"{}/{} ({})",
			env!("CARGO_PKG_NAME"),
			env!("CARGO_PKG_VERSION"),
			env!("CARGO_PKG_REPOSITORY")
		))
		.build()
		.unwrap()
});

fn get_all_bsps(bsp_folder: &Path) -> Bsps {
	let mut bsps = Bsps::new();
	for entry in std::fs::read_dir(bsp_folder).unwrap() {
		let entry = entry.unwrap();
		if let Some(ext) = entry.path().extension() {
			if ext.eq_ignore_ascii_case("bsp") {
				// lol
				let _ = bsps.insert(hex_to_hash(entry.path().file_stem().unwrap().to_str().unwrap()));
			}
		}
	}
	bsps
}

/*
fn get_bsps(bsp_folder: &Path, bsps: &Bsps) -> HashMap<> {
	let mut bsps = Bsps::new();
	for entry in std::fs::read_dir(bsp_folder).unwrap() {
		let entry = entry.unwrap();
		if let Some(ext) = entry.path().extension() {
			if ext.eq_ignore_ascii_case("bsp") {
				// lol
				let _ = bsps.insert(hex_to_hash(&entry.path().file_stem().unwrap().to_str().unwrap()));
			}
		}
	}
	bsps
}
*/

fn main() -> anyhow::Result<()> {
	unsafe {
		std::env::set_var("RUST_BACKTRACE", "full");
	}

	let rt = tokio::runtime::Runtime::new()?;

	rt.block_on(async { tokio::spawn(async_main()).await? })
}

async fn async_main() -> anyhow::Result<()> {
	// cargo flamegraph -- process

	let sanity_check_timestamp = |timestamp: i64| {
		assert!(timestamp > 2026_01_01_0000 && timestamp < 2027_01_01_0001);
		jiff::Timestamp::strptime("%Y%m%d%H%M%z", format!("{timestamp}+0000"))
	};

	let args = Cli::parse();
	match args.command {
		// bleh
		Commands::SemiManual { timestamp } => {
			let timestamp = sanity_check_timestamp(timestamp)?;
			base::semimanual::run(timestamp, None, None).await?;
		}
		Commands::ProcessAndTransfer => {
			base::process::run().await?;
			let _ = base::nodes::transfer()
				.join_all()
				.await
				.into_iter()
				.collect::<anyhow::Result<Vec<_>>>()?;
			cloudflare::purge_cache(None).await?;
		}
		// maps-cstrike
		Commands::Mapshasher {
			timestamp_fixer,
			skip_existing_hash,
			canon_clobber_check,
		} => {
			base::mapshasher::run(
				SETTINGS.dir_maps_cstrike.join("unprocessed/misc3.csv"),
				base::mapshasher::Mode::Manual,
				SETTINGS.dir_manualmaps.clone(),
				timestamp_fixer,
				skip_existing_hash,
				canon_clobber_check,
			)
			.await?;
		}
		Commands::Autogb => {
			// we use some of the processed csvs inside gbauto->mapshasher, so make sure they exist here...
			base::process::run().await?;
			gamebanana::auto::run().await?;
		}
		Commands::Process => base::process::run().await?,
		Commands::Gbdls => csv::fill_downloads(&SETTINGS.dir_gamebanana_scrape).await?,
		Commands::Cfpages => cloudflare::upload_pages().await?,
		Commands::TransferToNodes => {
			let _ = base::nodes::transfer()
				.join_all()
				.await
				.into_iter()
				.collect::<anyhow::Result<Vec<_>>>()?;
		}
		Commands::Ezcanon { name, hash } => {
			base::ezcanon::run(&name, &hash).await?;
		}
		// maps-cstrike-more
		Commands::Auto2 { timestamp } => {
			let timestamp = sanity_check_timestamp(timestamp)?;
			more::auto::run(None, timestamp).await?
		}
		Commands::Dumper => {
			let _ = more::dumper::dumpher().await?;
		}
		Commands::LumpChecksummer => {
			let bsps = get_all_bsps(&SETTINGS.dir_hashed);
			tokio::task::spawn_blocking(move || more::lump_checksummer::run(&bsps)).await??;
		}
		Commands::OriginalFilename => {
			let bsps = get_all_bsps(&SETTINGS.dir_hashed);
			tokio::task::spawn_blocking(move || more::original_mapname::run(&bsps)).await??;
		}
		Commands::Timestamper => {
			tokio::task::spawn_blocking(move || more::timestamper::run(None)).await??;
		}
		Commands::Venus => more::venus::upload().await?,
		Commands::Vscripter => {
			let bsps = get_all_bsps(&SETTINGS.dir_hashed);
			tokio::task::spawn_blocking(move || more::vscripter::run(&bsps)).await??;
		}
		// cloudflare
		Commands::PurgeCache => cloudflare::purge_cache(None).await?,
		Commands::SyncR2Bz2s => cloudflare::sync_r2_bz2s().await?,
	}

	Ok(())
}
