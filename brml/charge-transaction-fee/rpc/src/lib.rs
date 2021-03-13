// Copyright 2019-2021 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

use codec::{Codec, Decode};
use jsonrpc_core::{Error as RpcError, ErrorCode, Result as JsonRpcResult};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::Bytes;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, Zero},
};
use std::marker::PhantomData;
use std::sync::Arc;

pub use brml_charge_transaction_fee_rpc_runtime_api::ChargeTransactionFeeRuntimeApi as FeeRuntimeApi;
pub use pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi as TransactionPaymentRuntimeApi;

#[derive(Clone, Debug)]
pub struct ChargeTransactionFeeStruct<C, Block> {
    client: Arc<C>,
    _marker: PhantomData<Block>,
}

impl<C, Block> ChargeTransactionFeeStruct<C, Block> {
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: PhantomData,
        }
    }
}

#[rpc]
pub trait FeeRpcApi<BlockHash, CurrencyId, AccountId, Balance> {
    /// rpc method get balances by account id
    /// useage: curl -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "assets_getBalances", "params": [0, "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"]}' http://localhost:9933/
    #[rpc(name = "fee_get_token_and_amount")]
    fn get_fee_token_and_amount(
        &self,
        who: AccountId,
        encoded_xt: Bytes,
        at: Option<BlockHash>,
    ) -> JsonRpcResult<(CurrencyId, Balance)>;
}

/// Error type of this RPC api.
pub enum Error {
    /// The transaction was not decodable.
    DecodeError,
    /// The call to runtime failed.
    RuntimeError,
}

impl From<Error> for i64 {
    fn from(e: Error) -> i64 {
        match e {
            Error::RuntimeError => 1,
            Error::DecodeError => 2,
        }
    }
}

impl<C, Block, CurrencyId, AccountId, Balance>
    FeeRpcApi<<Block as BlockT>::Hash, CurrencyId, AccountId, Balance>
    for ChargeTransactionFeeStruct<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: FeeRuntimeApi<Block, CurrencyId, AccountId, Balance>
        + TransactionPaymentRuntimeApi<Block, Balance>,
    AccountId: Codec,
    CurrencyId: Codec,
    Balance: Codec + std::fmt::Display + std::ops::Add<Output = Balance> + sp_runtime::traits::Zero,
{
    fn get_fee_token_and_amount(
        &self,
        who: AccountId,
        encoded_xt: Bytes,
        at: Option<<Block as BlockT>::Hash>,
    ) -> JsonRpcResult<(CurrencyId, Balance)> {
        let api = self.client.runtime_api();
        let at = BlockId::<Block>::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let encoded_len = encoded_xt.len() as u32;

        let uxt: Block::Extrinsic = Decode::decode(&mut &*encoded_xt).map_err(|e| RpcError {
            code: ErrorCode::ServerError(Error::DecodeError.into()),
            message: "Unable to query fee details.".into(),
            data: Some(format!("{:?}", e).into()),
        })?;
        let fee_details = api
            .query_fee_details(&at, uxt, encoded_len)
            .map_err(|e| RpcError {
                code: ErrorCode::ServerError(Error::RuntimeError.into()),
                message: "Unable to query fee details.".into(),
                data: Some(format!("{:?}", e).into()),
            })?;

        let total_inclusion_fee: Balance = {
            if let Some(inclusion_fee) = fee_details.inclusion_fee {
                let base_fee = inclusion_fee.base_fee;
                let len_fee = inclusion_fee.len_fee;
                let adjusted_weight_fee = inclusion_fee.adjusted_weight_fee;

                base_fee + len_fee + adjusted_weight_fee
            } else {
                Zero::zero()
            }
        };

        api.get_fee_token_and_amount(&at, who, total_inclusion_fee)
            .map_err(|e| RpcError {
                code: ErrorCode::ServerError(Error::RuntimeError.into()),
                message: "Unable to query fee token and amount.".into(),
                data: Some(format!("{:?}", e).into()),
            })
    }
}