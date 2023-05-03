#![doc(html_logo_url = "https://raw.githubusercontent.com/AbstractSDK/assets/mainline/logo.svg")]
#![doc = include_str!("../../README.md")]
#![doc(test(attr(
    warn(unused),
    deny(warnings),
    // W/o this, we seem to get some bogus warning about `extern crate zbus`.
    allow(unused_extern_crates, unused),
)))]

mod contract;
#[cfg(feature = "daemon")]
mod daemon;
mod deploy;
mod error;
mod index_response;
#[cfg(feature = "interchain")]
mod interchain;
mod interface_traits;
#[cfg(feature = "daemon")]
mod keys;
mod mock;
mod state;
mod tx_handler;

pub use contract::Contract;
pub use cw_orch_contract_derive::{contract, interface};
pub use cw_orch_fns_derive::{ExecuteFns, QueryFns};
pub use deploy::{Deploy};
pub use error::CwOrcError;
pub use index_response::IndexResponse;
pub use interface_traits::{
    CallAs, ContractInstance, CwOrcExecute, CwOrcInstantiate, CwOrcMigrate, CwOrcQuery,
    CwOrcUpload, ExecuteableContract, InstantiateableContract, MigrateableContract,
    QueryableContract, Uploadable,
};

#[allow(deprecated)]
pub use mock::{
    core::{instantiate_custom_mock_env, instantiate_default_mock_env, Mock},
    state::MockState,
};
pub use state::{ChainState, StateInterface};
pub use tx_handler::{ChainUpload, TxHandler, TxResponse};
// re-export as it is used in the public API
pub use cosmwasm_std::{Addr, Coin, Empty};
pub use cw_multi_test::{custom_app, BasicApp, ContractWrapper};

#[cfg(feature = "daemon")]
pub use daemon::{
    artifacts_dir::ArtifactsDir,
    builder::DaemonBuilder,
    channel::DaemonChannel,
    core::Daemon,
    error::DaemonError,
    ibc_tracker, networks, queriers,
    traits::{MigrateHelpers, UploadHelpers},
    wasm_path::WasmPath,
    Wallet,
};

#[cfg(feature = "daemon")]
pub mod channel {
    pub use crate::daemon::channel::ChannelAccess;
}

#[cfg(feature = "interchain")]
pub use interchain::{hermes::Hermes, infrastructure::InterchainInfrastructure};

#[cfg(feature = "daemon")]
pub use ibc_chain_registry::{chain::ChainData as RegistryChainData, fetchable::Fetchable};

/// Signals a supported execution environment for CosmWasm contracts
pub trait CwEnv: TxHandler + ChainUpload + Clone {}
impl<T: TxHandler + ChainUpload + Clone> CwEnv for T {}
