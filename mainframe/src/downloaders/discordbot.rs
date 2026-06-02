use twilight_gateway::{Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt};

use crate::SETTINGS;

pub async fn lurk() {
	let mut shard = Shard::new(ShardId::ONE, SETTINGS.discord_bottoken.clone(), Intents::empty());

	while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
		if let Ok(Event::GatewayHeartbeatAck) = &item {
			continue;
		}
		println!("{item:#?}");
	}
}
