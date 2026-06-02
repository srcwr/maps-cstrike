// SPDX-License-Identifier: WTFPL

#[cfg(feature = "scraper")]
pub mod auto;
#[cfg(feature = "discordbot")]
pub mod discordbot;
pub mod types;

pub use types::GamebananaID;
