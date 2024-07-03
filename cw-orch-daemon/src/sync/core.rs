use std::{
    fmt::Debug,
    ops::DerefMut,
    sync::{RwLockReadGuard, RwLockWriteGuard},
};

use super::super::senders::Wallet;
use crate::{
    queriers::{Bank, CosmWasmBase, Node},
    senders::query::QuerySender,
    CosmTxResponse, DaemonAsyncBase, DaemonBuilder, DaemonError, DaemonState,
};
use cosmwasm_std::{Addr, Coin};
use cw_orch_core::{
    contract::{interface_traits::Uploadable, WasmPath},
    environment::{ChainInfoOwned, ChainState, DefaultQueriers, QueryHandler, TxHandler},
};
use cw_orch_traits::stargate::Stargate;
use serde::Serialize;
use tokio::runtime::Handle;
use tonic::transport::Channel;

use crate::senders::tx::TxSender;

pub type Daemon = DaemonBase<Wallet>;

#[derive(Clone)]
/**
Represents a blockchain node.
Is constructed with the [DaemonBuilder].

## Usage

```rust,no_run
use cw_orch_daemon::{Daemon, networks};
use tokio::runtime::Runtime;

let rt = Runtime::new().unwrap();
let daemon: Daemon = Daemon::builder(networks::JUNO_1)
    .build()
    .unwrap();
```
## Environment Execution

The Daemon implements [`TxHandler`] which allows you to perform transactions on the chain.

## Querying

Different Cosmos SDK modules can be queried through the daemon by calling the [`Daemon.query_client<Querier>`] method with a specific querier.
See [Querier](crate::queriers) for examples.
*/
pub struct DaemonBase<Sender> {
    pub(crate) daemon: DaemonAsyncBase<Sender>,
    /// Runtime handle to execute async tasks
    pub rt_handle: Handle,
}

impl<Sender> DaemonBase<Sender> {
    /// Get the daemon builder
    pub fn builder(chain: impl Into<ChainInfoOwned>) -> DaemonBuilder {
        DaemonBuilder::new(chain)
    }

    /// Get the mutable Sender object
    pub fn sender_mut(&self) -> RwLockWriteGuard<Sender> {
        self.daemon.sender_mut()
    }

    /// Get the channel configured for this Daemon
    pub fn sender(&self) -> RwLockReadGuard<Sender> {
        self.daemon.sender()
    }

    /// Flushes all the state related to the current chain
    /// Only works on Local networks
    pub fn flush_state(&mut self) -> Result<(), DaemonError> {
        self.daemon.flush_state()
    }

    /// Return the chain info for this daemon
    pub fn chain_info(&self) -> &ChainInfoOwned {
        self.daemon.chain_info()
    }
}

impl<Sender: QuerySender> DaemonBase<Sender> {
    /// Get the channel configured for this Daemon
    pub fn channel(&self) -> Channel {
        self.daemon.sender().channel()
    }

    /// Returns a new [`DaemonBuilder`] with the current configuration.
    /// **Does not copy the `Sender`**
    /// Does not consume the original [`Daemon`].
    pub fn rebuild(&self) -> DaemonBuilder {
        DaemonBuilder {
            state: Some(self.state()),
            chain: self.daemon.chain_info().clone(),
            deployment_id: Some(self.daemon.state.deployment_id.clone()),
            state_path: None,
            write_on_change: None,
            handle: Some(self.rt_handle.clone()),
            mnemonic: None,
        }
    }
}

// Helpers for Daemon with [`Wallet`] sender.
impl Daemon {
    pub fn sender_addr(&self) -> Addr {
        self.daemon.sender_addr()
    }

    /// Specifies wether authz should be used with this daemon
    pub fn authz_granter(&mut self, granter: impl ToString) -> &mut Self {
        self.sender_mut().set_authz_granter(granter.to_string());
        self
    }

    /// Specifies wether feegrant should be used with this daemon
    pub fn fee_granter(&mut self, granter: impl ToString) -> &mut Self {
        self.sender_mut().set_fee_granter(granter.to_string());
        self
    }
}

impl<Sender> ChainState for DaemonBase<Sender> {
    type Out = DaemonState;

    fn state(&self) -> Self::Out {
        self.daemon.state.clone()
    }
}

// Execute on the real chain, returns tx response
impl<Sender: TxSender> TxHandler for DaemonBase<Sender> {
    type Response = CosmTxResponse;
    type Error = DaemonError;
    type ContractSource = WasmPath;
    type Sender = Sender;

    fn sender(&self) -> Addr {
        self.daemon.sender_addr()
    }

    fn set_sender(&mut self, sender: Self::Sender) {
        let mut daemon_sender = self.daemon.sender_mut();
        (*daemon_sender.deref_mut()) = sender;
    }

    fn upload<T: Uploadable>(&self, uploadable: &T) -> Result<Self::Response, DaemonError> {
        self.rt_handle.block_on(self.daemon.upload(uploadable))
    }

    fn execute<E: Serialize>(
        &self,
        exec_msg: &E,
        coins: &[cosmwasm_std::Coin],
        contract_address: &Addr,
    ) -> Result<Self::Response, DaemonError> {
        self.rt_handle
            .block_on(self.daemon.execute(exec_msg, coins, contract_address))
    }

    fn instantiate<I: Serialize + Debug>(
        &self,
        code_id: u64,
        init_msg: &I,
        label: Option<&str>,
        admin: Option<&Addr>,
        coins: &[Coin],
    ) -> Result<Self::Response, DaemonError> {
        self.rt_handle.block_on(
            self.daemon
                .instantiate(code_id, init_msg, label, admin, coins),
        )
    }

    fn migrate<M: Serialize + Debug>(
        &self,
        migrate_msg: &M,
        new_code_id: u64,
        contract_address: &Addr,
    ) -> Result<Self::Response, DaemonError> {
        self.rt_handle.block_on(
            self.daemon
                .migrate(migrate_msg, new_code_id, contract_address),
        )
    }

    fn instantiate2<I: Serialize + Debug>(
        &self,
        code_id: u64,
        init_msg: &I,
        label: Option<&str>,
        admin: Option<&Addr>,
        coins: &[cosmwasm_std::Coin],
        salt: cosmwasm_std::Binary,
    ) -> Result<Self::Response, Self::Error> {
        self.rt_handle.block_on(
            self.daemon
                .instantiate2(code_id, init_msg, label, admin, coins, salt),
        )
    }
}

impl<Sender: TxSender> Stargate for DaemonBase<Sender> {
    fn commit_any<R>(
        &self,
        msgs: Vec<prost_types::Any>,
        memo: Option<&str>,
    ) -> Result<Self::Response, Self::Error> {
        self.rt_handle
            .block_on(
                self.sender_mut().commit_tx_any(
                    msgs.iter()
                        .map(|msg| cosmrs::Any {
                            type_url: msg.type_url.clone(),
                            value: msg.value.clone(),
                        })
                        .collect(),
                    memo,
                ),
            )
            .map_err(Into::into)
    }
}

impl<Sender: QuerySender> QueryHandler for DaemonBase<Sender> {
    type Error = DaemonError;

    fn wait_blocks(&self, amount: u64) -> Result<(), DaemonError> {
        self.rt_handle.block_on(self.daemon.wait_blocks(amount))?;

        Ok(())
    }

    fn wait_seconds(&self, secs: u64) -> Result<(), DaemonError> {
        self.rt_handle.block_on(self.daemon.wait_seconds(secs))?;

        Ok(())
    }

    fn next_block(&self) -> Result<(), DaemonError> {
        self.rt_handle.block_on(self.daemon.next_block())?;

        Ok(())
    }
}

impl<Sender: QuerySender> DefaultQueriers for DaemonBase<Sender> {
    type Bank = Bank;
    type Wasm = CosmWasmBase<Sender>;
    type Node = Node;
}
