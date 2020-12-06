// Copyright 2019-2020 Parity Technologies (UK) Ltd.
// This file is part of substrate-subxt.
//
// subxt is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// subxt is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with substrate-subxt.  If not, see <http://www.gnu.org/licenses/>.

//! Implements support for the pallet_contracts module.

use crate::{
    contracts::{
        Contracts,
        Gas,
    },
    frame::{
        balances::{
            Balances,
            BalancesEventsDecoder,
        },
        runtime_gateway::{
            CallStamp,
            ExecutionProofs,
            ExecutionStamp,
            TransferEntry,
        },
        system::{
            System,
            SystemEventsDecoder,
        },
    },
};
use codec::{
    Decode,
    Encode,
};
use core::marker::PhantomData;
use sp_core::H256;

/// The subset of the `pallet_contracts::Trait` that a client must implement.
#[module]
pub trait ContractsGateway: System + Balances {}

/// Stores the given binary Wasm code into the chain's storage and returns
/// its `codehash`.
/// You can instantiate contracts only with stored code.
#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct GatewayContractExecCall<'a, T: ContractsGateway> {
    /// Runtime marker.
    pub _runtime: PhantomData<T>,
    /// Address of the execution requester.
    pub requester: <T as System>::AccountId,
    /// Address of the target destination (of attached value transfer as the contract calls aren't possible on runtime gateway).
    pub target_dest: <T as System>::AccountId,
    /// Current phase of multistep execution.
    pub phase: u8,
    /// Wasm blob.
    pub code: &'a [u8],
    /// Value to transfer to the target_dest.
    #[codec(compact)]
    pub value: <T as Balances>::Balance,
    /// Gas limit.
    #[codec(compact)]
    pub gas_limit: Gas,
    /// Data to initialize the contract with.
    pub data: &'a [u8],
}

/// Multistep Call Execution phase after event.
///
/// Emitted upon successful execution of a multistep call, emitting the entire execution stamp via event.
#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct MultistepExecutePhaseSuccessEvent<T: ContractsGateway> {
    /// Stamp after successful execution phase.
    pub execution_stamp: ExecutionStamp,
    /// Runtime marker.
    pub _runtime: PhantomData<T>,
}

#[cfg(test)]
#[cfg(feature = "integration-tests")]
mod tests {
    use sp_keyring::AccountKeyring;

    use super::*;
    use crate::{
        balances::*,
        system::*,
        Client,
        ClientBuilder,
        ContractsTemplateRuntime,
        Error,
        ExtrinsicSuccess,
        PairSigner,
        Signer,
    };
    use sp_core::{
        crypto::AccountId32,
        sr25519::Pair,
    };
    use std::sync::atomic::{
        AtomicU32,
        Ordering,
    };

    static STASH_NONCE: std::sync::atomic::AtomicU32 = AtomicU32::new(0);

    struct TestContext {
        client: Client<ContractsTemplateRuntime>,
        signer: PairSigner<ContractsTemplateRuntime, Pair>,
        requester: PairSigner<ContractsTemplateRuntime, Pair>,
    }

    impl TestContext {
        async fn init() -> Self {
            env_logger::try_init().ok();

            let client = ClientBuilder::<ContractsTemplateRuntime>::new()
                .build()
                .await
                .expect("Error creating client");
            let mut stash: PairSigner<ContractsTemplateRuntime, Pair> =
                PairSigner::new(AccountKeyring::Alice.pair());
            let nonce = client
                .account(&stash.account_id(), None)
                .await
                .unwrap()
                .nonce;
            let local_nonce = STASH_NONCE.fetch_add(1, Ordering::SeqCst);

            stash.set_nonce(nonce + local_nonce);

            let signer = PairSigner::new(AccountKeyring::Alice.pair());
            let requester = PairSigner::new(AccountKeyring::Charlie.pair());

            TestContext {
                client,
                signer,
                requester,
            }
        }


        async fn gateway_contract_exec(
            &self,
        ) -> Result<MultistepExecutePhaseSuccessEvent<ContractsTemplateRuntime>, Error>
        {
            const CONTRACT: &str = r#"
                (module
                    (func (export "call"))
                    (func (export "deploy"))
                )
            "#;
            let code = wabt::wat2wasm(CONTRACT).expect("invalid wabt");
            use sp_core::Pair as _;
            let new_account = Pair::generate().0;
            let requester: AccountId32 = sp_keyring::AccountKeyring::Bob.to_account_id();
            let target_dest: AccountId32 = new_account.public().into();
            let phase: u8 = 0;
            let value: <ContractsTemplateRuntime as Balances>::Balance = 0;
            let gas: Gas = 500_000_000;
            let result = self
                .client
                .gateway_contract_exec_and_watch(
                    &self.signer,
                    requester,
                    target_dest,
                    phase, // phase = Execution
                    &code,
                    value, // value
                    gas,   // gas_limit
                    &[],   // input data
                )
                .await?;
            log::info!("gateway_contract_exec_and_watch res: {:?}", result);
            let execution_success_event =
                result.multistep_execute_phase_success()?.ok_or_else(|| {
                    Error::Other(
                        "Failed to find a MultistepExecutePhaseSuccess event".into(),
                    )
                })?;
            log::info!(
                "MultistepExecutePhaseSuccess execution_stamp: {:?}",
                execution_success_event.execution_stamp
            );

            Ok(execution_success_event)
        }
    }

    #[async_std::test]
    async fn tx_gateway_contract_exec() {
        let ctx = TestContext::init().await;
        let gateway_contract_exec_res = ctx.gateway_contract_exec().await;

        assert!(
            gateway_contract_exec_res.is_ok(),
            format!("Error calling gateway_contract_exec: {:?}", gateway_contract_exec_res)
        );
    }
}
