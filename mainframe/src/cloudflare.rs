// SPDX-License-Identifier: WTFPL
// Copyright 2025 rtldg <rtldg@protonmail.com>

use std::{path::Path, process::Stdio, time::Duration};

use rusty_s3::{Bucket, Credentials, S3Action, actions::ListObjectsV2};
use serde_json::{Value, json};
use tokio::task::JoinSet;

use crate::{Bsps, CLIENT, SETTINGS, hash_to_hex, hex_to_hash};

// https://docs.rs/rusty-s3/latest/rusty_s3/
pub(crate) async fn r2_upload(localpath: &Path, bucket: &str, remotepath: &str, content_type: &str) -> anyhow::Result<()> {
	let bucket = Bucket::new(
		format!("https://{}.r2.cloudflarestorage.com", SETTINGS.r2_account_id).parse()?,
		rusty_s3::UrlStyle::Path,
		bucket.to_string(),
		"auto",
	)?;
	let credentials = Credentials::new(SETTINGS.s3_access_key_id.clone(), SETTINGS.s3_access_key_secret.clone());
	let signed_url = bucket
		.put_object(Some(&credentials), remotepath)
		.sign(Duration::from_secs(120));
	CLIENT
		.put(signed_url)
		.header("content-type", content_type)
		// TODO: stream this & also have a progress callback...
		.body(tokio::fs::read(localpath).await?)
		.send()
		.await?;
	Ok(())
}

pub(crate) async fn purge_cache(urls: Option<&[&str]>) -> anyhow::Result<()> {
	let purge_url = format!("https://api.cloudflare.com/client/v4/zones/{}/purge_cache", SETTINGS.cf_zone);
	let response: Value = CLIENT
		.request(reqwest::Method::POST, purge_url)
		.bearer_auth(SETTINGS.cf_purgetoken.as_str())
		.json(&match urls {
			Some(urls) => json!({"files": urls}),
			None => json!({"purge_everything": true}),
		})
		.send()
		.await?
		.json()
		.await?;

	if response.get("success") != Some(&Value::Bool(true)) {
		dbg!(response);
		anyhow::bail!("failed to purge cache");
	}

	Ok(())
}

pub(crate) async fn upload_pages() -> anyhow::Result<()> {
	#[cfg(windows)]
	const NPX: &str = "npx.cmd";
	#[cfg(not(windows))]
	const NPX: &str = "npx";

	let a = tokio::spawn(async {
		let dir = dunce::canonicalize(SETTINGS.dir_maps_cstrike.join("processed/check.fastdl.me"))?;
		tokio::process::Command::new(NPX)
			.args([
				"--yes",
				"wrangler",
				"pages",
				"deploy",
				"--commit-dirty=true",
				"--project-name",
				"check-fastdl",
				"--branch",
				"main",
				".",
			])
			.current_dir(dir)
			.stdin(Stdio::null())
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status()
			.await
	});
	let b = tokio::spawn(async {
		let dir = dunce::canonicalize(SETTINGS.dir_maps_cstrike.join("processed/fastdl.me"))?;
		tokio::process::Command::new(NPX)
			.args([
				"--yes",
				"wrangler",
				"pages",
				"deploy",
				"--commit-dirty=true",
				"--project-name",
				"fdl",
				"--branch",
				"master",
				".",
			])
			.current_dir(dir)
			.stdin(Stdio::null())
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status()
			.await
	});
	anyhow::ensure!(a.await??.success(), "cloudflare pages deployment of check.fastdl.me failed");
	anyhow::ensure!(b.await??.success(), "cloudflare pages deployment of fastdl.me failed");
	Ok(())
}

pub(crate) async fn sync_r2_bz2s() -> anyhow::Result<()> {
	let start_time = std::time::Instant::now();

	let localhashes = tokio::spawn(async {
		let mut hashes = Bsps::new();
		let mut dir = tokio::fs::read_dir(&SETTINGS.dir_hashed).await?;

		while let Some(entry) = dir.next_entry().await? {
			let name = entry.file_name();
			let name = name.to_str().unwrap();
			// hardcoded sha1 digest sizes...
			if name.len() != (40 + ".bsp.bz2".len()) || !name.ends_with(".bsp.bz2") {
				continue;
			}
			hashes.insert(hex_to_hash(&name[..40]));
		}

		anyhow::Result::<_, anyhow::Error>::Ok(hashes)
	});

	// we have a total of 55,000~ maps right now
	//   prefix of "hashed/" + a single hex digit (0 to f)
	// = 3,437~ objects per prefix
	// = 4~ requests per prefix (because ListObjectsV2 returns 1000~ objects)
	// = 48 total requests maybe hopefully
	let max_requests_per_prefix = 10; // should be enough :)

	let mut request_set = JoinSet::new();
	for shard in "0123456789abcdef".chars() {
		request_set.spawn(async move {
			let mut hashes = Bsps::new();

			let bucket = Bucket::new(
				format!("https://{}.r2.cloudflarestorage.com", SETTINGS.r2_account_id).parse()?,
				rusty_s3::UrlStyle::Path,
				SETTINGS.s3_bucket_hashed.to_owned(),
				"auto",
			)?;
			let credentials = Credentials::new(SETTINGS.s3_access_key_id.clone(), SETTINGS.s3_access_key_secret.clone());

			let mut list_objects_v2 = bucket.list_objects_v2(Some(&credentials));
			list_objects_v2.with_prefix(format!("hashed/{shard}"));

			for _i in 0..max_requests_per_prefix {
				//println!("{shard} {_i}");
				let signed_url = list_objects_v2.sign(Duration::from_secs(12));
				let resp = CLIENT.get(signed_url).send().await?.bytes().await?;
				let resp = ListObjectsV2::parse_response(&resp)?;

				for object in resp.contents {
					if object.key.ends_with(".bsp.bz2") {
						// We love hardcoding sha1 hexadecimal digest sizes and string lengths.
						// We also do it this way because it's shrimple.
						// R2 gives a url/percent -encoded key (which means we can't split as easily on "hashed/").
						let hash = &object.key[object.key.len() - 48..object.key.len() - 8];
						//println!("{hash}");
						hashes.insert(hex_to_hash(hash));
					}
				}

				if let Some(token) = resp.next_continuation_token {
					list_objects_v2.with_continuation_token(token);
				} else {
					break;
				}
			}

			anyhow::Result::<_, anyhow::Error>::Ok(hashes)
		});
	}

	let mut r2hashes = Bsps::new();
	while let Some(res) = request_set.join_next().await {
		r2hashes.append(&mut res??);
	}

	let localhashes = localhashes.await??;

	let to_upload = localhashes.difference(&r2hashes).collect::<Vec<_>>();

	for hash in &to_upload {
		println!("need to upload {}", hash_to_hex(hash));
	}

	println!("sync_r2_bz2s diffing took {}s", start_time.elapsed().as_secs_f64());

	for hash in &to_upload {
		let hash = hash_to_hex(hash);
		let localpath = SETTINGS.dir_hashed.join(format!("{hash}.bsp.bz2"));
		let remotepath = format!("hashed/{hash}.bsp.bz2");
		println!("uploading {hash}.bsp.bz2...");
		r2_upload(&localpath, &SETTINGS.s3_bucket_hashed, &remotepath, "application/x-bzip").await?;
	}

	Ok(())
}
