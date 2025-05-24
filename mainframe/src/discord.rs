// SPDX-License-Identifier: WTFPL
// Copyright 2025 rtldg <rtldg@protonmail.com>

use crate::{CLIENT, SETTINGS};

pub(crate) async fn webhook(ping: bool, message: &str) -> anyhow::Result<()> {
	let content = if ping {
		format!("{} {message}", SETTINGS.discord_ping)
	} else {
		message.to_string()
	};
	CLIENT
		.post(SETTINGS.discord_webhook.as_str())
		.json(&serde_json::json!({
			"username": SETTINGS.discord_username.as_str(),
			"embeds": [],
			"content": content,
		}))
		.send()
		.await?;
	Ok(())
}
