// SPDX-License-Identifier: WTFPL

#[cfg(feature = "scraping")]
pub mod auto;
#[cfg(feature = "discordbot")]
pub mod discordbot;
pub mod types;

pub use types::GamebananaID;
