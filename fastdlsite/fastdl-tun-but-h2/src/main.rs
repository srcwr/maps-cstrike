// SPDX-License-Identifier: WTFPL

//#![forbid(unsafe_code)]

const HTTPS: bool = !cfg!(debug_assertions);

use std::{
	net::SocketAddr,
	path::PathBuf,
	str::FromStr,
	sync::{Arc, LazyLock},
	time::Duration,
};

use axum::{
	Json, Router,
	body::Body,
	extract::State,
	http::StatusCode,
	response::Redirect,
	routing::{get, post},
};
use axum_server::tls_rustls::RustlsConfig;
use http::{HeaderName, HeaderValue};
use rustls::ServerConfig;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tap::Tap;
use tower::ServiceBuilder;
use tower_http::{
	sensitive_headers::SetSensitiveRequestHeadersLayer,
	services::ServeFile,
	set_header::SetResponseHeaderLayer,
	trace::{DefaultMakeSpan, TraceLayer},
};

const SHA1_DIGEST_LEN: usize = 40;

static TLS_CONFIG: LazyLock<RustlsConfig> = LazyLock::new(|| {
	let subject_alt_names = vec!["127.0.0.1".to_string(), "fastdl-tun-but-h2".to_string()]; // these actually don't matter...
	let pair = rcgen::generate_simple_self_signed(subject_alt_names).unwrap();
	RustlsConfig::from_config(Arc::new(
		ServerConfig::builder()
			.with_no_client_auth()
			.with_single_cert(vec![pair.cert.der().to_owned()], pair.signing_key.try_into().unwrap())
			.unwrap()
			.tap_mut(|c| c.alpn_protocols = vec![b"h2".to_vec()]),
	))
});

fn main() -> anyhow::Result<()> {
	unsafe {
		std::env::set_var("RUST_BACKTRACE", "full");
	}

	tracing_subscriber::fmt()
		.json()
		// This allows you to use, e.g., `RUST_LOG=info` or `RUST_LOG=debug`
		// when running the app to set log levels.
		.with_env_filter(
			tracing_subscriber::EnvFilter::try_from_default_env()
				.or_else(|_| tracing_subscriber::EnvFilter::try_new("fastd-tun-but-h2=trace,tower_http=trace"))
				.unwrap(),
		)
		.init();

	tracing::info!("HTTPS = {HTTPS}");

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

	tracing::info!("starting check.fastdl.me form-handler (0.0.0.0:9002)");

	if HTTPS {
		axum_server::bind_rustls(SocketAddr::from(([0, 0, 0, 0], 9002)), TLS_CONFIG.clone())
			.serve(app.into_make_service())
			.await?;
	} else {
		axum::serve(
			tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], 9002))).await?,
			app,
		)
		.await?;
	}

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
	let forms_dir = PathBuf::from(&std::env::var("FORMS_DIR_INNER").unwrap_or_else(|_| "../forms".to_owned()));
	let fullpath = forms_dir.join(jiff::Timestamp::now().strftime("%Y%m%d_%H%M%S.txt").to_string());
	tokio::fs::write(fullpath, form.content)
		.await
		.map_err(|_e| StatusCode::INTERNAL_SERVER_ERROR)?;
	Ok(Json(FormYay { yip: "pie" }))
}

#[derive(Clone)]
struct MapsHandlerState {
	pool: sqlx::Pool<sqlx::Sqlite>,
}

static NODE_ID: LazyLock<String> = LazyLock::new(|| std::env::var("NODE_ID").unwrap_or_else(|_| "unset".to_string()));

async fn main_fastdl_me() -> anyhow::Result<()> {
	let processed_dir =
		PathBuf::from(&std::env::var("PROCESSED_DIR_INNER").unwrap_or_else(|_e| "../../processed".to_owned()));

	let state = Arc::new(MapsHandlerState {
		pool: SqlitePoolOptions::new()
			.max_lifetime(Some(Duration::from_secs(15)))
			.connect_with(
				SqliteConnectOptions::new()
					.read_only(true)
					.filename(processed_dir.join("maps-lite.db")),
			)
			.await?,
	});

	let make_app = || {
		let sensitive_headers: Vec<_> = std::env::var("SENSITIVE_HEADERS")
			.unwrap_or_default()
			.split(',')
			.filter(|s| !s.is_empty())
			.chain(["CF-Connecting-IP", "X-Real-IP", "X-Forwarded-For"])
			.map(|s| HeaderName::from_str(s).unwrap())
			.collect();

		Router::new()
			.route("/hashed/{hash}/{mapname_with_ext}", get(hashed_2))
			.route("/h2/{hash}/{mapname_with_ext}", get(hashed_2))
			.route("/hashed/{hash_with_ext}", get(hashed_1))
			.route("/mapsredir/{mapname_with_ext}", get(maps_1))
			.route("/maps/{mapname_with_ext}", get(maps_1))
			.route("/m2/{mapname_with_ext}", get(maps_1))
			.route("/h2/", get(|| async { Redirect::to("https://main.fastdl.me/hashed/") }))
			.route("/m2/", get(|| async { Redirect::to("https://main.fastdl.me/maps/") }))
			.route_service(
				"/hashed/",
				ServeFile::new(processed_dir.join("main.fastdl.me/hashed_index.html"))
					.precompressed_gzip()
					.precompressed_zstd(),
			)
			.route_service(
				"/maps/",
				ServeFile::new(processed_dir.join("main.fastdl.me/maps_index.html"))
					.precompressed_gzip()
					.precompressed_zstd(),
			)
			.fallback_service(
				tower_http::services::ServeDir::new(processed_dir.join("main.fastdl.me"))
					.precompressed_gzip()
					.precompressed_zstd(),
			)
			.layer(
				ServiceBuilder::new()
					.layer(axum::middleware::map_response(tag_node))
					.layer(SetSensitiveRequestHeadersLayer::new(sensitive_headers))
					.layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::new().include_headers(true)))
					.layer(axum::middleware::from_fn(strip_index_dot_html))
					.layer(axum::middleware::from_fn(nav_short_circuit))
					.layer(SetResponseHeaderLayer::overriding(
						http::header::CONTENT_TYPE,
						|resp: &http::Response<axum::body::Body>| {
							if let Some(v) = resp.headers().get(http::header::CONTENT_TYPE)
								&& let Ok(v) = v.to_str()
								&& (v == "text/plain" || v == "text/csv")
							{
								Some(HeaderValue::from_static("text/plain; charset=utf-8"))
							} else {
								None
							}
						},
					)),
			)
	};

	let mut tasks = tokio::task::JoinSet::new();

	tasks.spawn({
		let app = make_app().with_state(state.clone());
		async move {
			tracing::info!("starting main.fastdl.me handler (0.0.0.0:9001)");

			if HTTPS {
				axum_server::bind_rustls(SocketAddr::from(([0, 0, 0, 0], 9001)), TLS_CONFIG.clone())
					.serve(app.into_make_service())
					.await
			} else {
				axum::serve(
					tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], 9001))).await?,
					app,
				)
				.await
			}
		}
	});

	// Sample: 7443=http://.site.nfoservers.com/server/;7444=https://test.example.org/
	for (port, redirect_url) in std::env::var("THIRDPARTY_REDIRECTS")
		.unwrap_or_default()
		.split(';')
		.filter_map(|s| s.split_once('='))
	{
		let port = port.parse()?;
		tracing::info!("starting thirdparty main.fastdl.me handler (0.0.0.0:{port}) ({redirect_url})");
		let app = make_app()
			.with_state(state.clone())
			.layer(axum::middleware::from_fn_with_state(
				redirect_url.to_owned().leak() as &'static str,
				thirdparty_redirecter,
			));

		tasks.spawn(async move {
			if HTTPS {
				axum_server::bind_rustls(SocketAddr::from(([0, 0, 0, 0], port)), TLS_CONFIG.clone())
					.serve(app.into_make_service())
					.await
			} else {
				axum::serve(
					tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))).await?,
					app,
				)
				.await
			}
		});
	}

	while let Some(t) = tasks.join_next().await {
		t??;
	}

	Ok(())
}

async fn thirdparty_redirecter(
	State(redirect_url): State<&'static str>,
	req: axum::extract::Request,
	next: axum::middleware::Next,
) -> axum::response::Response {
	let path = req.uri().path();

	if path.ends_with('/') {
		return axum::response::Response::builder()
			.status(StatusCode::SEE_OTHER)
			.header("Location", format!("https://main.fastdl.me{path}"))
			.body(Body::empty())
			.unwrap();
	}

	let path = path.to_owned();

	let response = next.run(req).await;

	if response.status() == StatusCode::NOT_FOUND {
		let redirect_url = redirect_url.trim_end_matches('/');
		return axum::response::Response::builder()
			.status(StatusCode::SEE_OTHER)
			.header("Location", format!("{redirect_url}{path}"))
			.body(Body::empty())
			.unwrap();
	}

	response
}

async fn nav_short_circuit(req: axum::extract::Request, next: axum::middleware::Next) -> axum::response::Response {
	if req.uri().path().starts_with("/maps/") && req.uri().path().ends_with(".nav.bz2") {
		return axum::response::Response::new(include_bytes!("dummy.nav.bz2").as_slice().into());
	}

	next.run(req).await
}

async fn strip_index_dot_html(req: axum::extract::Request, next: axum::middleware::Next) -> axum::response::Response {
	if let Some(s) = req.uri().path().strip_suffix("index.html")
		&& s.ends_with('/')
	{
		return axum::response::Response::builder()
			.status(StatusCode::TEMPORARY_REDIRECT)
			.header("Location", s)
			.body(Body::empty())
			.unwrap();
	}

	next.run(req).await
}

async fn tag_node<B>(mut response: axum::response::Response<B>) -> axum::response::Response<B> {
	response
		.headers_mut()
		.insert("X-Node-ID", HeaderValue::from_str(&NODE_ID).unwrap());
	response
}

fn smells_like_sha1(s: &str) -> bool {
	s.len() == SHA1_DIGEST_LEN && s.chars().all(|c| c.is_ascii_hexdigit())
}

async fn hashed_1(axum::extract::Path(hash_with_ext): axum::extract::Path<String>) -> Result<Redirect, StatusCode> {
	let hash = hash_with_ext.strip_suffix(".bsp.bz2").unwrap();
	if smells_like_sha1(hash) {
		// TODO: actually check if it exists in the database... but who cares since this never hits as the cloudflare worker is handling this
		Ok(Redirect::to(&format!("https://main.fastdl.me/hashed/{hash}.bsp.bz2")))
	} else {
		Err(StatusCode::NOT_FOUND)
	}
}

async fn hashed_2(
	axum::extract::Path((hash, _mapname_with_ext)): axum::extract::Path<(String, String)>,
) -> Result<Redirect, StatusCode> {
	if smells_like_sha1(&hash) {
		// TODO: actually check if it exists in the database... but who cares since this never hits as the cloudflare worker is handling this
		Ok(Redirect::to(&format!("https://main.fastdl.me/hashed/{hash}.bsp.bz2")))
	} else {
		Err(StatusCode::NOT_FOUND)
	}
}

async fn maps_1(
	axum::extract::Path(mut mapname_with_ext): axum::extract::Path<String>,
	State(state): State<Arc<MapsHandlerState>>,
) -> Result<Redirect, StatusCode> {
	let is_bz2 = mapname_with_ext.ends_with(".bz2");
	if is_bz2 {
		mapname_with_ext.truncate(mapname_with_ext.len() - ".bz2".len());
	}
	if !mapname_with_ext.ends_with(".bsp") {
		return Err(StatusCode::NOT_FOUND);
	}
	mapname_with_ext.truncate(mapname_with_ext.len() - ".bsp".len());

	if !is_bz2 {
		// TODO: log
		return Err(StatusCode::NOT_FOUND);
	}

	mapname_with_ext.make_ascii_lowercase();
	let mapname = mapname_with_ext;

	let hash = if smells_like_sha1(&mapname) {
		mapname
	} else {
		sqlx::query_scalar("SELECT sha1, MAX(filesize_bz2) fbz2 FROM maps_canon WHERE mapname = $1;")
			.bind(&mapname)
			.fetch_one(&state.pool)
			.await
			.map_err(|_e| StatusCode::INTERNAL_SERVER_ERROR)?
	};

	if hash.is_empty() {
		Err(StatusCode::NOT_FOUND)
	} else {
		Ok(Redirect::to(&format!("https://mainr2.fastdl.me/hashed/{hash}.bsp.bz2")))
	}
}
