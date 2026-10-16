// SPDX-License-Identifier: MIT

pub mod driver;
pub mod exa;
pub mod manager;
pub mod pool;
pub mod types;

pub use driver::WebSearchDriver;
pub use exa::ExaSearchDriver;
pub use manager::WebSearchManager;
pub use pool::{mask_secret, PooledKey, SearchKeyPool};
pub use types::{
    MaskedKeyInfo, SearchError, SearchOptions, SearchResultItem, SearchResults, SearchType,
    SelectionStrategy,
};
