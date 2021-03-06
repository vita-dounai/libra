// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    code_cache::module_cache::ModuleCache,
    data_cache::BlockDataCache,
    process_txn::execute::ExecutedTransaction,
    txn_executor::{TransactionExecutor, LIBRA_SYSTEM_MODULE},
};
use lazy_static::lazy_static;
use libra_types::{
    block_metadata::BlockMetadata,
    identifier::Identifier,
    transaction::TransactionOutput,
    vm_error::{StatusCode, VMStatus},
};
use vm::{
    gas_schedule::{CostTable, GasAlgebra, GasUnits},
    transaction_metadata::TransactionMetadata,
};
use vm_runtime_types::value::Value;

lazy_static! {
    static ref BLOCK_PROLOGUE: Identifier = Identifier::new("block_prologue").unwrap();
}

pub(crate) fn process_block_metadata<'alloc, P>(
    block_metadata: BlockMetadata,
    module_cache: P,
    data_cache: &mut BlockDataCache<'_>,
) -> TransactionOutput
where
    P: ModuleCache<'alloc>,
{
    // TODO: How should we setup the metadata here? A couple of thoughts here:
    // 1. We might make the txn_data to be poisoned so that reading anything will result in a panic.
    // 2. The most important consideration is figuring out the sender address.  Having a notion of a
    //    "null address" (probably 0x0...0) that is prohibited from containing modules or resources
    //    might be useful here.
    // 3. We set the max gas to a big number just to get rid of the potential out of gas error.
    let mut txn_data = TransactionMetadata::default();

    txn_data.max_gas_amount = GasUnits::new(std::u64::MAX);
    // TODO: We might need a non zero cost table here so that we can at least bound the execution
    //       time by a reasonable amount.
    let gas_schedule = CostTable::zero();

    let mut txn_executor =
        TransactionExecutor::new(&module_cache, &gas_schedule, data_cache, txn_data);
    let result = if let Ok((id, timestamp, previous_vote, proposer)) = block_metadata.into_inner() {
        let args = vec![
            Value::u64(timestamp),
            Value::byte_array(id),
            Value::byte_array(previous_vote),
            Value::address(proposer),
        ];
        txn_executor.execute_function(&LIBRA_SYSTEM_MODULE, &BLOCK_PROLOGUE, args)
    } else {
        Err(VMStatus::new(StatusCode::MALFORMED))
    };
    match result {
        Ok(_) => txn_executor.transaction_cleanup(vec![]),
        Err(err) => ExecutedTransaction::discard_error_output(err),
    }
}
