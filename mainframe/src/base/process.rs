// SPDX-License-Identifier: WTFPL
// Copyright 2025 rtldg <rtldg@protonmail.com>

use std::{
	collections::{BTreeMap, BTreeSet},
	fmt::Write as _,
	io::Write as _,
	path::Path,
	sync::{Arc, LazyLock},
	time::Instant,
};

use anyhow::Context;
use dashmap::{DashMap, DashSet};
use flate2::{Compression, write::GzEncoder};
use indoc::{formatdoc, writedoc};
use rayon::iter::{ParallelBridge, ParallelIterator};
use regex::Regex;
use rusqlite::{OpenFlags, Transaction};
use thousands::Separable;

use crate::{
	SETTINGS,
	base::copytree,
	csv::{CanonCsvRow, ProcessedCsvRow, UnprocessedCsvRow},
	gamebanana::GamebananaID,
	normalize_mapname,
};

#[derive(Default)]
struct UnprocessedCsvsInfo {
	gamebanana: DashMap<String, (GamebananaID, GamebananaID)>,
	links: DashMap<String, String>,
	unique: DashSet<UnprocessedCsvRow>,
}

fn get_gamebanana_info(s: &str) -> Option<(GamebananaID, GamebananaID)> {
	static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([1-9]+[0-9]*)_(\d+)").unwrap());
	if let Some((_, [modid, downloadid])) = RE.captures(s).map(|c| c.extract()) {
		return Some((modid.parse().ok()?, downloadid.parse().ok()?));
	}
	None
}

fn glob_unprocessed_csvs(pattern: &str) -> anyhow::Result<UnprocessedCsvsInfo> {
	let info = UnprocessedCsvsInfo::default();
	glob::glob(&SETTINGS.dir_maps_cstrike.join(pattern).to_string_lossy())?
		.par_bridge()
		.try_for_each(|path| {
			let path = path?;
			let mut in_csv = csv::Reader::from_path(path)?;
			for row in in_csv.deserialize::<UnprocessedCsvRow>() {
				let mut row = row?;
				if row.mapname.starts_with('#') {
					continue;
				}
				row.mapname = normalize_mapname(&row.mapname);
				if let Some(note) = row.note.take() {
					if note.starts_with("http://") || note.starts_with("https://") {
						let previous_link = info.links.insert(row.sha1.clone(), note);
						if let Some(previous_link) = previous_link {
							anyhow::bail!("previous link for {} should be None but was {}", row.sha1, previous_link);
						}
					} else if let Some(gbids) = get_gamebanana_info(&note) {
						// TODO: (low) people LOVE reposting maps in map-packs so we can't rely on hashes having a single gbid pair...
						let _ = info.gamebanana.insert(row.sha1.clone(), gbids);
					}
				}
				let _ = info.unique.insert(row);
			}
			anyhow::Result::<(), anyhow::Error>::Ok(())
		})?;
	Ok(info)
}

fn glob_filters(pattern: &str, mapset: &DashSet<UnprocessedCsvRow>) -> anyhow::Result<()> {
	glob::glob(&SETTINGS.dir_maps_cstrike.join(pattern).to_string_lossy())?
		.par_bridge()
		.try_for_each(|path| {
			let path = path?;
			let mut in_csv = csv::Reader::from_path(path)?;
			for row in in_csv.deserialize::<UnprocessedCsvRow>() {
				let mut row = row?;
				if row.mapname.starts_with('#') {
					continue;
				}
				row.mapname = normalize_mapname(&row.mapname);
				row.note = None;
				let _ = mapset.remove(&row);
			}
			Ok(())
		})
}

pub(crate) async fn run() -> anyhow::Result<()> {
	let start = Instant::now();

	let conn = tokio_rusqlite::Connection::open_with_flags(
		"file::memory:?cache=shared",
		OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_URI,
	)
	.await?;

	conn.call(|conn| {
		conn.execute_batch(
			"
			CREATE TABLE maps_unfiltered (mapname TEXT NOT NULL, filesize INT NOT NULL, filesize_bz2 INT NOT NULL, sha1 TEXT NOT NULL);
			CREATE TABLE maps_canon (mapname TEXT NOT NULL, filesize INT NOT NULL, filesize_bz2 INT NOT NULL, sha1 TEXT NOT NULL);
			CREATE TABLE maps_czarchasm (mapname TEXT NOT NULL, filesize INT NOT NULL, filesize_bz2 INT NOT NULL, sha1 TEXT NOT NULL);
			CREATE TABLE maps_ksfthings (mapname TEXT NOT NULL, filesize INT NOT NULL, filesize_bz2 INT NOT NULL, sha1 TEXT NOT NULL);
			CREATE TABLE gamebanana (sha1 TEXT NOT NULL, gamebananaid INT NOT NULL, gamebananafileid INT NOT NULL);
			CREATE TABLE links (sha1 TEXT NOT NULL, url TEXT NOT NULL);
			",
		)?;
		Ok(())
	})
	.await?;

	// TODO: (low) remerge maps table & add `canon` column to table?

	println!("unprocesed csvs {}", Instant::now().duration_since(start).as_secs_f64());
	let UnprocessedCsvsInfo {
		gamebanana,
		links,
		unique,
	} = glob_unprocessed_csvs("unprocessed/*.csv")?;
	println!("czar csvs {}", Instant::now().duration_since(start).as_secs_f64());
	let UnprocessedCsvsInfo {
		gamebanana: _,
		links: _,
		unique: czarchasm_unique,
	} = glob_unprocessed_csvs("unprocessed/hashed_bsps_czar_p*.csv")?;
	println!("ksf csvs {}", Instant::now().duration_since(start).as_secs_f64());
	let UnprocessedCsvsInfo {
		gamebanana: _,
		links: _,
		unique: ksfthings,
	} = glob_unprocessed_csvs("unprocessed/ksf - github.com OuiSURF Surf_Maps.csv")?;

	println!("filter csvs {}", Instant::now().duration_since(start).as_secs_f64());
	let unfiltered = unique.clone();
	glob_filters("filters/*.csv", &unique)?;
	glob_filters("filters/custom/czarchasm_filter.csv", &czarchasm_unique)?;

	conn.call(move |conn| {
		println!(
			"inserting maps into sqlite db {}",
			Instant::now().duration_since(start).as_secs_f64()
		);
		let tx = conn.transaction()?;
		let insert_maps = |tx: &Transaction, table: &str, rows: &DashSet<UnprocessedCsvRow>| -> rusqlite::Result<()> {
			let mut stmt = tx.prepare(&format!("INSERT INTO {table} VALUES(?,?,?,?);"))?;
			for row in rows.iter() {
				let _ = stmt.execute((row.mapname.as_str(), row.filesize, row.filesize_bz2, row.sha1.as_str()))?;
			}
			drop(stmt);
			Ok(())
		};
		insert_maps(&tx, "maps_unfiltered", &unfiltered)?;
		insert_maps(&tx, "maps_canon", &unique)?;
		insert_maps(&tx, "maps_czarchasm", &czarchasm_unique)?;
		insert_maps(&tx, "maps_ksfthings", &ksfthings)?;
		let mut stmt = tx.prepare("INSERT INTO gamebanana VALUES(?,?,?);")?;
		for r in gamebanana.iter() {
			let (sha1, (modid, downloadid)) = r.pair();
			let _ = stmt.execute((sha1.as_str(), modid, downloadid))?;
		}
		drop(stmt);
		let mut stmt = tx.prepare("INSERT INTO links VALUES(?,?);")?;
		for r in links.iter() {
			let (sha1, link) = r.pair();
			let _ = stmt.execute((sha1.as_str(), link.as_str()))?;
		}
		drop(stmt);
		tx.commit()?;
		Ok(())
	})
	.await?;

	// CREATE INDEX's down here after we've inserted so there's less index churn...
	let create_indexes_task = conn.call(move |conn| {
		println!("creating indexes {}", Instant::now().duration_since(start).as_secs_f64());
		conn.execute_batch(
			"
			CREATE INDEX mapnameu ON maps_unfiltered(mapname);
			CREATE INDEX sha1m on maps_unfiltered(sha1);
			CREATE INDEX mapnamec ON maps_canon(mapname);
			CREATE INDEX sha1c on maps_canon(sha1);
			CREATE INDEX mapnamecz ON maps_czarchasm(mapname);
			CREATE INDEX sha1cz on maps_czarchasm(sha1);
			CREATE INDEX mapnameksf ON maps_ksfthings(mapname);
			CREATE INDEX sha1ksf on maps_ksfthings(sha1);
			CREATE INDEX sha1g on gamebanana(sha1);
			CREATE INDEX sha1o on links(sha1);
			",
		)?;
		Ok(())
	});
	let canon_rows = csv::Reader::from_path(SETTINGS.dir_maps_cstrike.join("canon.csv"))?
		.deserialize::<CanonCsvRow>()
		.collect::<csv::Result<Vec<_>>>()
		.context("fucked up canon.csv again...")?;
	create_indexes_task.await?;
	conn.call(move |conn| {
		println!("canonizing db {}", Instant::now().duration_since(start).as_secs_f64());
		let tx = conn.transaction()?;
		let mut stmt = tx.prepare("DELETE FROM maps_canon WHERE mapname = ? AND sha1 != ?;")?;
		for row in &canon_rows {
			if row.mapname.starts_with('#') {
				continue;
			}
			let _ = stmt.execute((row.mapname.as_str(), row.sha1.as_str()))?;
		}
		drop(stmt);
		tx.commit()?;
		Ok(())
	})
	.await?;

	let mapsdb = SETTINGS.dir_maps_cstrike.join("processed/maps.db");
	let _ = tokio::fs::remove_file(&mapsdb).await;
	let vacuum_future = conn.call(move |conn| {
		println!("vacuuming to maps.db {}", Instant::now().duration_since(start).as_secs_f64());
		let mut stmt = conn.prepare("VACUUM INTO ?;")?;
		let _ = stmt.execute((mapsdb.to_string_lossy(),))?;
		println!("vacuum done {}", Instant::now().duration_since(start).as_secs_f64());
		Ok(())
	});

	//println!("recently added {}", Instant::now().duration_since(start).as_secs_f64());
	let mut recently_added = vec![];
	let mut recently_added_csv = csv::Reader::from_path(SETTINGS.dir_maps_cstrike.join("recently_added.csv"))?;
	for row in recently_added_csv.deserialize::<UnprocessedCsvRow>() {
		let mut row = row?;
		if row.mapname.starts_with('#') {
			continue;
		}
		row.mapname = normalize_mapname(&row.mapname);
		if let Some((modid, _)) = get_gamebanana_info(row.note.as_deref().unwrap_or("")) {
			if let Some(recently_added_note) = row.recently_added_note {
				row.recently_added_note = Some(format!(
					"<a href=\"https://gamebanana.com/mods/{modid}\">gamebanana</a> - {recently_added_note}"
				));
			} else {
				row.recently_added_note = Some(format!("<a href=\"https://gamebanana.com/mods/{modid}\">gamebanana</a>"));
			}
		} else if row.recently_added_note.is_none() {
			row.recently_added_note = Some(String::new());
		}
		recently_added.push(row);
		if recently_added.len() > 154 {
			break;
		}
	}
	let recently_added = Arc::new(recently_added);

	async fn reset_dir<P: AsRef<Path>>(from: P, to: P) -> anyhow::Result<()> {
		let from = from.as_ref();
		let to = to.as_ref();
		if tokio::fs::try_exists(to).await? {
			tokio::fs::remove_dir_all(to).await?;
		}
		//println!("reset_dir {} -> {}", from.display(), to.display());
		copytree::copy_with_mtime(from, to).await?;
		Ok(())
	}

	//println!("check.fastdl.me {}", Instant::now().duration_since(start).as_secs_f64());
	reset_dir(
		SETTINGS.dir_maps_cstrike.join("fastdlsite/check.fastdl.me"),
		SETTINGS.dir_maps_cstrike.join("processed/check.fastdl.me"),
	)
	.await?;
	let things = conn
		.call(|conn| {
			let mut stmt = conn.prepare("SELECT mapname, filesize FROM maps_unfiltered;")?;
			let mut rows = stmt.query(())?;
			let mut things = BTreeMap::<String, Vec<usize>>::new();
			while let Some(row) = rows.next()? {
				let mapname: String = row.get(0)?;
				let filesize: usize = row.get(1)?;
				things.entry(mapname).or_default().push(filesize);
			}
			Ok(things)
		})
		.await?;
	let mut json = vec![];
	serde_json::to_writer(&mut json, &things)?;
	tokio::fs::write(SETTINGS.dir_maps_cstrike.join("processed/mapnames_and_filesizes.json"), json).await?;

	//println!("fastdl.me {}", Instant::now().duration_since(start).as_secs_f64());
	reset_dir(
		SETTINGS.dir_maps_cstrike.join("fastdlsite/fastdl.me"),
		SETTINGS.dir_maps_cstrike.join("processed/fastdl.me"),
	)
	.await?;
	tokio::fs::write(
		SETTINGS.dir_maps_cstrike.join("processed/fastdl.me/index.html"),
		tokio::fs::read_to_string(SETTINGS.dir_maps_cstrike.join("fastdlsite/fastdl.me/index.html"))
			.await?
			.replace(
				"<!-- embed the privacy policy here -->",
				&tokio::fs::read_to_string(SETTINGS.dir_maps_cstrike.join("fastdlsite/embedded-privacy-policy.html")).await?,
			),
	)
	.await?;

	println!("main.fastdl.me {}", Instant::now().duration_since(start).as_secs_f64());
	reset_dir(
		SETTINGS.dir_maps_cstrike.join("fastdlsite/main.fastdl.me"),
		SETTINGS.dir_maps_cstrike.join("processed/main.fastdl.me"),
	)
	.await?;
	reset_dir(
		SETTINGS.dir_maps_cstrike.join("../fastdl_opendir/materials"),
		SETTINGS.dir_maps_cstrike.join("processed/main.fastdl.me/materials"),
	)
	.await?;
	reset_dir(
		SETTINGS.dir_maps_cstrike.join("../fastdl_opendir/sound"),
		SETTINGS.dir_maps_cstrike.join("processed/main.fastdl.me/sound"),
	)
	.await?;
	copytree::copy_with_mtime(
		SETTINGS.dir_maps_cstrike.join("LICENSE"),
		SETTINGS.dir_maps_cstrike.join("processed/main.fastdl.me/WTFPL.txt"),
	)
	.await?;
	copytree::copy_with_mtime(
		SETTINGS.dir_maps_cstrike.join("LICENSE"),
		SETTINGS.dir_maps_cstrike.join("processed/fastdl.me/WTFPL.txt"),
	)
	.await?;

	fn create_html_csv_txt_files(
		table: &str,
		outfilename: &str,
		canon: bool,
		title_plaintext: &str,
		title_html: &str,
		sqlwhere: &str,
		recently_added: Option<Arc<Vec<UnprocessedCsvRow>>>,
		txt_as_urls: bool,
	) -> anyhow::Result<()> {
		let outfilename = Path::new(outfilename);

		//println!("{title}");
		let conn = rusqlite::Connection::open_with_flags(
			"file::memory:?cache=shared",
			OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
		)?;

		let (number_of_maps, unpacked_size, bz2_size): (usize, usize, usize) = conn.query_row(
			&format!(
				"
			SELECT COUNT(*), SUM(s1), SUM(s2)
			FROM (
				SELECT SUM(filesize) s1, SUM(filesize_bz2) s2
				FROM {table} {sqlwhere} GROUP BY sha1
			);"
			),
			(),
			|row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
		)?;
		//println!("  header  {table}");
		let mut index_html = formatdoc!(
			r##"
			<!DOCTYPE html>
			<html>
			<head>
			<meta http-equiv="content-type" content="text/html; charset=utf-8">
			<meta name="viewport" content="width=device-width">
			<title>fastdl.me {title_plaintext}</title>
			{}
			<h1>fastdl.me {title_html}</h1>
			page hit count: <img height=14 width=92 alt=hc src="https://hc.fastdl.me/hc/{}.jpg"><br>
			<h2><a href="https://fastdl.me">homepage</a></h2>
			<h3>Number of maps: {number_of_maps}</h3>
			<h3>Unpacked size: {} BYTES</h3>
			<h3>BZ2 size: {} BYTES</h3>
			links to other versions of this list: <a href="https://{}.txt">txt</a> / <a href="https://{}.csv">csv</a>
			<br>&nbsp;
		"##,
			std::fs::read_to_string(SETTINGS.dir_maps_cstrike.join("index_top.html"))?,
			outfilename.file_stem().unwrap().to_string_lossy(),
			unpacked_size.separate_with_commas(),
			bz2_size.separate_with_commas(),
			outfilename.to_string_lossy(),
			outfilename.to_string_lossy()
		);
		if let Some(recently_added) = &recently_added {
			//println!("  recently added");
			writedoc!(
				&mut index_html,
				r##"
				<br>
				<h2>Recently added:</h2>
				<a href="https://github.com/srcwr/maps-cstrike/commits/master">(full commit history)</a>
				<table id="recentlyadded">
				<thead>
				<tr>
				<th style="width:1%">Map name</th>
				<th style="width:1%">SHA-1 Hash</th>
				<th style="width:15%">Note</th>
				<th style="width:2%">Date added</th>
				</tr>
				</thead>
				<tbody>
			"##
			)?;
			//<th style="width:1%">List of packed files</th>
			for row in recently_added.iter() {
				writedoc!(
					&mut index_html,
					r##"
					<tr>
					<td><a href="#">{}</a></td>
					<td>{}</td>
					<td>{}</td>
					<td>{}</td>
					</tr>
				"##,
					html_escape::encode_safe(&row.mapname),
					row.sha1,
					row.recently_added_note.as_deref().unwrap_or_default(),
					row.datetime.as_ref().map(|s| s.as_str()).unwrap_or_default()
				)?;
			}
			//<td><a href="https://github.com/srcwr/maps-cstrike-more/blob/master/filelist/{}.csv">{}</a></td>
			write!(&mut index_html, "</tbody></table>")?;
		}

		//println!("  map tables!  {table}");
		writedoc!(
			&mut index_html,
			r##"
			<h4>(sorting is slow... you have been warned...)</h4>
			<table id="list" class="sortable">
			<thead>
			<tr>
			<th style="width:1%">Map name</th>
			<th style="width:5%">SHA-1 Hash</th>
			<th style="width:5%">Size bsp</th>
			<th style="width:5%">Size bz2</th>
			<th style="width:5%">Page</th>
			</tr>
			</thead>
			<tbody>
		"##
		)?;

		let mut outtext = String::new();

		let mut unfiltered_hashes = if table == "maps_unfiltered" {
			Some(BTreeSet::<String>::new())
		} else {
			None
		};

		let mut outcsv = csv::Writer::from_writer(vec![]);

		let (groupby, fzy) = if canon {
			("GROUP BY mapname", "MAX(filesize_bz2)")
		} else {
			("", "filesize_bz2")
		};

		let mut stmt = conn.prepare(&format!(
			"
			SELECT mapname, filesize, {fzy}, m.sha1, gamebananaid, url
			FROM {table} m
			LEFT JOIN gamebanana g ON g.sha1 = m.sha1
			LEFT JOIN links l ON l.sha1 = m.sha1
			{sqlwhere}
			{groupby}
			ORDER BY mapname;
		"
		))?;
		//println!("  sql rows... {table}");
		let mut rows = stmt.query(())?;
		while let Some(row) = rows.next()? {
			let mapname = row.get_ref(0)?.as_str()?;
			let filesize = row.get(1)?;
			let filesize_bz2 = row.get(2)?;
			let sha1 = row.get_ref(3)?.as_str()?;
			let gbid: Option<GamebananaID> = row.get(4)?;
			let mut link: Option<String> = row.get(5)?;
			let mut htmllink = String::new();

			if let Some(link) = &link {
				htmllink = format!("<td><a href=\"{link}\">clickme</a></td>");
			} else if let Some(gbid) = gbid {
				if gbid != 0 {
					link = Some(format!("https://gamebanana.com/mods/{gbid}"));
					htmllink = format!("<td><a href=\"https://gamebanana.com/mods/{gbid}\">{gbid}</a></td>");
				}
			}
			if txt_as_urls {
				writeln!(&mut outtext, "http://main.fastdl.me/hashed/{sha1}/{mapname}.bsp.bz2")?;
			} else if canon {
				writeln!(&mut outtext, "{mapname}")?;
			} else {
				let _ = unfiltered_hashes.as_mut().unwrap().insert(sha1.to_string());
			}

			outcsv.serialize(ProcessedCsvRow {
				mapname,
				sha1,
				filesize,
				filesize_bz2,
				url: &link.unwrap_or_default(),
			})?;

			writedoc!(
				&mut index_html,
				"<tr>\
				<td><a href=\"#\">{}</a></td>\
				<td>{sha1}</td>\
				<td>{filesize}</td>\
				<td>{filesize_bz2}</td>\
				{htmllink}\
				</tr>\
				",
				html_escape::encode_safe(mapname)
			)?;
		}

		//println!("  writing!  {table}");

		tokio::runtime::Handle::current().block_on(tokio::spawn({
			let outfilename = Arc::new(outfilename.to_string_lossy().to_string());
			//let table = table.to_string();
			async move {
				let mut futures = tokio::task::JoinSet::new();

				let bottom_html = tokio::spawn(tokio::fs::read_to_string(SETTINGS.dir_maps_cstrike.join("index_bottom.html")));

				futures.spawn({
					let outfilename = Arc::clone(&outfilename);
					async move {
						tokio::fs::write(
							SETTINGS.dir_maps_cstrike.join(format!("processed/{}.csv", outfilename)),
							outcsv.into_inner()?,
						)
						.await?;
						Ok(())
					}
				});

				futures.spawn({
					let outfilename = Arc::clone(&outfilename);
					async move {
						if let Some(unfiltered_hashes) = &unfiltered_hashes {
							for hash in unfiltered_hashes {
								writeln!(&mut outtext, "{hash}")?;
							}
						}
						tokio::fs::write(
							SETTINGS.dir_maps_cstrike.join(format!("processed/{}.txt", outfilename)),
							outtext,
						)
						.await?;
						Ok(())
					}
				});

				index_html.push_str(&bottom_html.await??);

				//tokio::fs::write(SETTINGS.dir_maps_cstrike.join(format!("processed/{outfilename}.orig")), index_html.as_bytes()).await?;

				/*
				//println!("  minifying...  {table}");
				let mut cfg = minify_html_onepass::Cfg::new();
				cfg.minify_js = false; // minify-js has various bugs so we have to disable it for now -- check back when there's a version newer than 0.6.0
				cfg.minify_css = true;
				let minified =
					Arc::new(tokio::task::spawn_blocking(move || minify_html_onepass::copy(index_html.as_bytes(), &cfg)).await??);
				*/
				let minified = Arc::new(index_html);

				futures.spawn({
					let outfilename = Arc::clone(&outfilename);
					let minified = Arc::clone(&minified);
					async move {
						tokio::fs::write(
							SETTINGS.dir_maps_cstrike.join(format!("processed/{outfilename}")),
							minified.as_bytes(),
						)
						.await?;
						Ok(())
					}
				});

				futures.spawn_blocking({
					let outfilename = Arc::clone(&outfilename);
					let minified = Arc::clone(&minified);
					move || -> anyhow::Result<()> {
						let mut e = GzEncoder::new(Vec::with_capacity(4_000_000), Compression::best());
						e.write_all(minified.as_bytes())?;
						std::fs::write(
							SETTINGS.dir_maps_cstrike.join(format!("processed/{outfilename}.gz")),
							e.finish()?,
						)?;
						Ok(())
					}
				});

				while let Some(res) = futures.join_next().await {
					res??;
				}

				anyhow::Result::<(), anyhow::Error>::Ok(())
			}
		}))??;

		Ok(())
	}

	println!(
		"making pages & writing {}",
		Instant::now().duration_since(start).as_secs_f64()
	);
	let mut futures = tokio::task::JoinSet::new();
	futures.spawn_blocking({
		let recently_added = Some(Arc::clone(&recently_added));
		move || {
			create_html_csv_txt_files(
				"maps_unfiltered",
				"main.fastdl.me/hashed_index.html",
				false,
				"hashed/unfiltered maps",
				"hashed/unfiltered maps",
				"",
				recently_added,
				false,
			)
		}
	});

	futures.spawn_blocking({
		let recently_added = Some(Arc::clone(&recently_added));
		move || {
			create_html_csv_txt_files(
				"maps_canon",
				"main.fastdl.me/maps_index.html",
				true,
				"canon/filtered maps",
				"canon/filtered maps",
				"",
				recently_added,
				false,
			)
		}
	});

	futures.spawn_blocking({
		let recently_added = Some(Arc::clone(&recently_added));
		move || {
			create_html_csv_txt_files(
				"maps_canon",
				"main.fastdl.me/69.html",
				true,
				"movement maps (mostly)",
				"movement maps (mostly)",
				"WHERE mapname LIKE 'bh%' OR mapname LIKE 'xc\\_%' ESCAPE '\\' OR mapname LIKE 'kz%' OR mapname LIKE 'surf%' OR mapname LIKE 'tsurf%' OR mapname LIKE 'trikz%' OR mapname LIKE 'jump%' OR mapname LIKE 'climb%' OR mapname LIKE 'fu\\_%' ESCAPE '\\' OR mapname LIKE '%hop%' OR mapname LIKE '%hns%'",
				recently_added,
				false,
			)
		}
	});

	futures.spawn_blocking(move || {
		create_html_csv_txt_files(
			"maps_czarchasm",
			"main.fastdl.me/maps_czarchasm.html",
			true,
			"mirror of maps from czarchasm.club",
			"mirror of maps from <a href=\"https://czarchasm.club/\">czarchasm.club</a>",
			"",
			None,
			true,
		)
	});

	futures.spawn_blocking(
		move || {
			// This is prone to breakage but fuck it...
			let last_ksfthings_update = std::fs::metadata(SETTINGS.dir_maps_cstrike.join("unprocessed/ksf - github.com OuiSURF Surf_Maps.csv"))?.modified()?;
			let last_ksfthings_update: jiff::Timestamp = last_ksfthings_update.try_into()?;
			let last_ksfthings_update = last_ksfthings_update.strftime("%Y-%m-%d").to_string();

			create_html_csv_txt_files(
				"maps_ksfthings",
				"main.fastdl.me/maps_ksfthings.html",
				true,
				&format!(
					"mirror of ksf maps from https://github.com/OuiSURF/Surf_Maps (up till {last_ksfthings_update})"
				),
				&format!(
					"mirror of ksf maps<br>from <a href=\"https://github.com/OuiSURF/Surf_Maps\">https://github.com/OuiSURF/Surf_Maps</a><br>(up till {last_ksfthings_update})"
				),
				"",
				None,
				true,
			)
		}
	);

	vacuum_future.await?;

	while let Some(res) = futures.join_next().await {
		res??;
	}

	println!("done! {}\n\n", Instant::now().duration_since(start).as_secs_f64());

	Ok(())
}
