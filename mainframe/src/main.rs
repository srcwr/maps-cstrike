// SPDX-License-Identifier: WTFPL
// Copyright 2022, 2025-2026 rtldg <rtldg@protonmail.com>

mod base; // maps-cstrike
#[cfg(feature = "scraper")]
mod cloudflare;
mod csv;
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
	#[cfg(feature = "scraper")]
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
	},
	#[cfg(feature = "scraper")]
	Autogb,
	Process,
	#[cfg(feature = "scraper")]
	Gbdls,
	#[cfg(feature = "scraper")]
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
	#[cfg(feature = "scraper")]
	Venus,
	Vscripter,
	// cloudflare
	#[cfg(feature = "scraper")]
	PurgeCache,
	#[cfg(feature = "scraper")]
	SyncR2Bz2s,
}

#[cfg(feature = "scraper")]
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
	#[cfg(feature = "scraper")]
	pub cf_zone: String,
	/// a token that has `Cache Purge:Purge` permissions on your zone
	#[cfg(feature = "scraper")]
	pub cf_purgetoken: String,
	/// a token with Workers perms
	#[cfg(feature = "scraper")]
	pub cf_pagestoken: String,
	/// fuck documenting
	#[cfg(feature = "scraper")]
	pub buckets: HashMap<String, BucketSettings>,
	/// the account id used in r2 (which is the same as the cloudflare "account" id)
	#[cfg(feature = "scraper")]
	pub r2_account_id: String,

	/// the discord webhook to post new downloads to
	#[cfg(feature = "scraper")]
	pub discord_webhook: String,
	/// a ping identifier. e.g. "<@&123123123123>" to ping a role
	#[cfg(feature = "scraper")]
	pub discord_ping: String,
	/// the discord username used for messages posted via webhook
	#[cfg(feature = "scraper")]
	pub discord_username: String,
	/// the bot token used for member bots...
	#[cfg(feature = "discordbot")]
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
	#[cfg(feature = "scraper")]
	pub gb_categories: Vec<GamebananaID>,
	/// Max of 50.
	#[cfg(feature = "scraper")]
	pub gb_maxperpage: NonZeroUsize,
	/// Non-zero value up to `gb_maxperpage`.
	#[cfg(feature = "scraper")]
	pub gb_perpage: NonZeroUsize,
	/// How many items to fetch.
	#[cfg(feature = "scraper")]
	pub gb_numtofetch: NonZeroUsize,
	/// Which item to start at...
	#[cfg(feature = "scraper")]
	pub gb_itemoffset: usize,
	/// Added because caching is ruining my life...
	#[cfg(feature = "scraper")]
	pub gb_walkto: usize,

	/// How long to wait after an error before looping autogb.  A few minutes is a good idea.
	#[cfg(feature = "scraper")]
	pub gb_wait_time_after_errors: f32,
	/// How many seconds to wait before loop autogb.  Multiple minutes is a good idea.
	#[cfg(feature = "scraper")]
	pub gb_wait_time_for_looping: f32,

	/// Proxy to use when pulling from gamebanana
	#[cfg(feature = "scraper")]
	pub proxy: Option<String>,
}

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

#[cfg(feature = "scraper")]
static PROXIED_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
	let c = reqwest::ClientBuilder::new()
		.connect_timeout(Duration::from_secs(10))
		.read_timeout(Duration::from_secs(120))
		.timeout(Duration::from_secs(140))
		.user_agent(format!(
			"{}/{} ({})",
			env!("CARGO_PKG_NAME"),
			env!("CARGO_PKG_VERSION"),
			env!("CARGO_PKG_REPOSITORY")
		));

	let c = if let Some(proxy) = &SETTINGS.proxy {
		c.proxy(reqwest::Proxy::all(proxy).unwrap())
	} else {
		c
	};

	c.build().unwrap()
});

#[cfg(feature = "scraper")]
static NOPROXY_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
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
		assert!(timestamp > 2026_03_20_0000 && timestamp < 2027_01_01_0001);
		jiff::Timestamp::strptime("%Y%m%d%H%M%z", format!("{timestamp}+0000"))
	};

	let args = Cli::parse();
	match args.command {
		// bleh
		#[cfg(feature = "scraper")]
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
			#[cfg(feature = "scraper")]
			cloudflare::purge_cache(None).await?;
		}
		// maps-cstrike
		Commands::Mapshasher {
			timestamp_fixer,
			skip_existing_hash,
		} => {
			base::mapshasher::run(
				SETTINGS.dir_maps_cstrike.join("unprocessed/misc3.csv"),
				base::mapshasher::Mode::Manual,
				SETTINGS.dir_manualmaps.clone(),
				timestamp_fixer,
				skip_existing_hash,
			)
			.await?;
		}
		#[cfg(feature = "scraper")]
		Commands::Autogb => {
			// we use some of the processed csvs inside gbauto->mapshasher, so make sure they exist here...
			base::process::run().await?;
			gamebanana::auto::run().await?;
		}
		Commands::Process => base::process::run().await?,
		#[cfg(feature = "scraper")]
		Commands::Gbdls => csv::fill_downloads(&SETTINGS.dir_gamebanana_scrape).await?,
		#[cfg(feature = "scraper")]
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
			more::auto::run(None, timestamp).await?;
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
		#[cfg(feature = "scraper")]
		Commands::Venus => more::venus::upload().await?,
		Commands::Vscripter => {
			let bsps = get_all_bsps(&SETTINGS.dir_hashed);
			tokio::task::spawn_blocking(move || more::vscripter::run(&bsps)).await??;
		}
		// cloudflare
		#[cfg(feature = "scraper")]
		Commands::PurgeCache => cloudflare::purge_cache(None).await?,
		#[cfg(feature = "scraper")]
		Commands::SyncR2Bz2s => cloudflare::sync_r2_bz2s().await?,
	}

	Ok(())
}
