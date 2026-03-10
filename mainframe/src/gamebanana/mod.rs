// SPDX-License-Identifier: WTFPL

#[cfg(feature = "scraper")]
pub mod auto;
#[cfg(feature = "discord")]
pub mod discordbot;
pub mod types;

pub use types::GamebananaID;
