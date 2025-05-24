// SPDX-License-Identifier: WTFPL
// Copyright 2025 rtldg <rtldg@protonmail.com>

use anyhow::Context;
use tokio::task::JoinSet;

use crate::SETTINGS;

pub(crate) fn transfer() -> JoinSet<anyhow::Result<()>> {
	let mut set = JoinSet::new();

	for nodeset in &SETTINGS.cmd_transfer_to_nodes {
		let nodeset = nodeset.clone();
		set.spawn(async move {
			for mut cmd_and_args in nodeset {
				let cmd = cmd_and_args.remove(0);
				// TODO: mute these fucking stdout/stderr...
				let status = tokio::process::Command::new(&cmd)
					.args(cmd_and_args)
					.status()
					.await
					.context(cmd.clone())?;
				if !status.success() {
					anyhow::bail!("failed with {} on {}", status.code().unwrap_or_default(), cmd);
				}
			}
			Ok(())
		});
	}

	set
}
