// SPDX-License-Identifier:
// Copyright

//#![forbid(unsafe_code)]

use std::{path::PathBuf, sync::Arc, time::Duration};

use axum::{
	Json, Router,
	extract::State,
	http::StatusCode,
	response::{IntoResponse, Redirect},
	routing::{get, post},
};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

const SHA1_DIGEST_LEN: usize = 40;

fn main() -> anyhow::Result<()> {
	unsafe {
		std::env::set_var("RUST_BACKTRACE", "full");
	}

	//tracing_subscriber::fmt::init();

	tokio::runtime::Runtime::new().unwrap().block_on(async {
		let mut tasks = tokio::task::JoinSet::new();
		tasks.spawn(check_fastdl_me_forms());
		tasks.spawn(main_fastdl_me());

		while let Some(t) = tasks.join_next().await {
			t??;
		}

		Ok(())
	})
}

async fn check_fastdl_me_forms() -> anyhow::Result<()> {
	let form_post_path = std::env::var("FORM_POST_PATH").unwrap_or_else(|_| "/form".to_owned());

	let app = Router::new().route(&form_post_path, post(check_fastdl_me_form_upload));
	let listener = tokio::net::TcpListener::bind("0.0.0.0:9002").await.unwrap();
	axum::serve(listener, app).await?;

	Ok(())
}

#[derive(serde::Deserialize)]
struct FormUpload {
	content: String,
}
#[derive(serde::Serialize)]
struct FormYay {
	yip: &'static str,
}
async fn check_fastdl_me_form_upload(Json(form): Json<FormUpload>) -> Result<Json<FormYay>, StatusCode> {
	let forms_dir = PathBuf::from(&std::env::var("FORMS_DIR").unwrap_or_else(|_| "../forms".to_owned()));
	let fullpath = forms_dir.join(jiff::Timestamp::now().strftime("%Y%m%d_%H%M%S.txt").to_string());
	let _ = tokio::fs::write(fullpath, form.content)
		.await
		.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
	Ok(Json(FormYay { yip: "pie" }))
}

#[derive(Clone)]
struct MapsHandlerState {
	pool: sqlx::Pool<sqlx::Sqlite>,
}

async fn main_fastdl_me() -> anyhow::Result<()> {
	let state = MapsHandlerState {
		pool: SqlitePoolOptions::new()
			.max_lifetime(Some(Duration::from_secs(15)))
			.connect_with(
				SqliteConnectOptions::new().read_only(true).filename(
					PathBuf::from(&std::env::var("PROCESSED_DIR").unwrap_or_else(|_| "../../processed".to_owned()))
						.join("maps-lite.db"),
				),
			)
			.await?,
	};

	let app = Router::new()
		.route("/hashed/{hash}/{mapname_with_ext}", get(hashed_2))
		.route("/h2/{hash}/{mapname_with_ext}", get(hashed_2))
		.route("/hashed/{hash_with_ext}", get(hashed_1))
		.route("/mapsredir/{mapname_with_ext}", get(maps_1))
		.route("/maps/{mapname_with_ext}", get(maps_1))
		.route("/m2/{mapname_with_ext}", get(maps_1))
		.with_state(Arc::new(state));

	let listener = tokio::net::TcpListener::bind("0.0.0.0:9001").await?;
	axum::serve(listener, app).await?;
	Ok(())
}

fn smells_like_sha1(s: &str) -> bool {
	s.len() == SHA1_DIGEST_LEN && s.chars().all(|c| c.is_ascii_hexdigit())
}

async fn hashed_1(axum::extract::Path(hash_with_ext): axum::extract::Path<String>) -> Result<Redirect, StatusCode> {
	let hash = hash_with_ext.strip_suffix(".bsp.bz2").unwrap();
	if smells_like_sha1(hash) {
		// TODO: actually check if it exists in the database... but who cares since this is just being fetched by the cloudflare worker...
		Ok(Redirect::to(&format!("https://main.fastdl.me/hashed/{hash}.bsp.bz2")))
	} else {
		Err(StatusCode::NOT_FOUND)
	}
}

async fn hashed_2(
	axum::extract::Path((hash, _mapname_with_ext)): axum::extract::Path<(String, String)>,
) -> Result<Redirect, StatusCode> {
	if smells_like_sha1(&hash) {
		// TODO: actually check if it exists in the database... but who cares since this is just being fetched by the cloudflare worker...
		Ok(Redirect::to(&format!("https://main.fastdl.me/hashed/{hash}.bsp.bz2")))
	} else {
		Err(StatusCode::NOT_FOUND)
	}
}

async fn maps_1(
	axum::extract::Path(mut mapname_with_ext): axum::extract::Path<String>,
	State(state): State<Arc<MapsHandlerState>>,
) -> Result<Redirect, (StatusCode, String)> {
	mapname_with_ext.truncate(mapname_with_ext.len() - ".bsp.bz2".len());
	mapname_with_ext.make_ascii_lowercase();
	let mapname = mapname_with_ext;

	let hash;

	if smells_like_sha1(&mapname) {
		hash = mapname;
	} else {
		hash = sqlx::query_scalar("SELECT sha1, MAX(filesize_bz2) fbz2 FROM maps_canon WHERE mapname = $1;")
			.bind(&mapname)
			.fetch_one(&state.pool)
			.await
			.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "sql query failed".to_owned()))?;
	}

	Ok(Redirect::to(&format!("https://mainr2.fastdl.me/hashed/{hash}.bsp.bz2")))
}
