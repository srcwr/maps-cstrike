// SPDX-License-Identifier: WTFPL
// Copyright 2025 rtldg <rtldg@protonmail.com>

use std::{path::Path, time::SystemTime};

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("couldn't open file")]
	OpenFile,
	#[error("couldn't set modified time")]
	SetModifiedTime,
	#[error("couldn't create directories")]
	CreateDirectory,
	#[error("failed to copy file")]
	CopyFile,
	#[error("failed to read directory")]
	ReadDirectory,
	#[error("failed to read metadata")]
	Metadata,
}

async fn set_mtime(path: &Path, mtime: SystemTime, is_dir: bool) -> Result<(), Error> {
	if is_dir {
		Ok(())
	} else {
		let path = path.to_owned();
		tokio::task::spawn_blocking(move || {
			std::fs::File::options()
				.write(true)
				.open(path)
				.map_err(|_| Error::OpenFile)?
				.set_modified(mtime)
				.map_err(|_| Error::SetModifiedTime)?;
			Ok(())
		})
		.await
		.unwrap()
	}
}

pub(crate) async fn copy_with_mtime<P: AsRef<Path>>(from: P, to: P) -> anyhow::Result<()> {
	let from = from.as_ref();
	let to = to.as_ref();

	if from.is_file() {
		tokio::fs::copy(from, to).await.map_err(|_| Error::CopyFile)?;
		let time = tokio::fs::metadata(from).await.map_err(|_| Error::Metadata)?;
		let modified = time.modified().map_err(|_| Error::Metadata)?;
		set_mtime(to, modified, false).await?;
		return Ok(());
	}

	tokio::fs::create_dir_all(to).await.map_err(|_| Error::CreateDirectory)?;

	let mut dir = tokio::fs::read_dir(from).await.map_err(|_| Error::ReadDirectory)?;
	let mut futures = tokio::task::JoinSet::new();
	while let Some(entry) = dir.next_entry().await.map_err(|_| Error::ReadDirectory)? {
		let path = entry.path();
		//dbg!(&path);
		let metadata = entry.metadata().await.map_err(|_| Error::Metadata)?;
		let modified = metadata.modified().map_err(|_| Error::Metadata)?;

		let newpath = to.join(path.file_name().unwrap());
		//dbg!(&newpath);

		if entry.file_type().await.map_err(|_| Error::Metadata)?.is_dir() {
			let newpath_clone = newpath.to_path_buf();
			Box::pin(copy_with_mtime(path, newpath_clone)).await?;
			set_mtime(&newpath, modified, true).await?;
		} else {
			futures.spawn(async move {
				tokio::fs::copy(&path, &newpath).await.map_err(|_| Error::CopyFile)?;
				set_mtime(&newpath, modified, false).await?;
				anyhow::Result::<(), anyhow::Error>::Ok(())
			});
		}
	}

	while let Some(res) = futures.join_next().await {
		res??;
	}

	Ok(())
}
