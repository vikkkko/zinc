//!
//! The contract resource POST method `call` module.
//!

pub mod error;
pub mod request;

use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

use actix_web::http::StatusCode;
use actix_web::web;
use num_old::BigUint;
use num_old::Zero;
use serde_json::json;
use serde_json::Value as JsonValue;

use zksync::operations::SyncTransactionHandle;
use zksync_eth_signer::PrivateKeySigner;
use zksync_types::tx::ZkSyncTx;

use zinc_build::Value as BuildValue;
use zinc_vm::Bn256;
use zinc_vm::ContractInput;
use zinc_zksync::Transaction;
use zinc_zksync::TransactionMsg;

use crate::database::model::field::select::Input as FieldSelectInput;
use crate::response::Response;
use crate::shared_data::SharedData;
use crate::storage::Storage;

use self::error::Error;
use self::request::Body as RequestBody;
use self::request::Query as RequestQuery;

///
/// The HTTP request handler.
///
/// Sequence:
/// 1. Get the contract from the in-memory cache.
/// 2. Extract the called method from its metadata and check if it is mutable.
/// 3. Parse the method input arguments.
/// 4. Get the contract storage from data sources and convert it to the Zinc VM representation.
/// 5. Run the method on the Zinc VM.
/// 6. Extract the storage with the updated state from the Zinc VM.
/// 7. Create a transactions array from the client and contract transfers.
/// 8. Send the transactions to zkSync and store its handles.
/// 9. Wait for all transactions to be committed.
/// 10. Update the contract storage state in the database.
/// 11. Send the contract method execution result back to the client.
///
pub async fn handle(
    app_data: web::Data<Arc<RwLock<SharedData>>>,
    query: web::Query<RequestQuery>,
    body: web::Json<RequestBody>,
) -> crate::Result<JsonValue, Error> {
    let query = query.into_inner();
    let body = body.into_inner();
    log::debug!("body:{:?}", body);
    let postgresql = app_data
        .read()
        .expect(zinc_const::panic::SYNCHRONIZATION)
        .postgresql
        .clone();

    log::debug!(
        "Calling method `{}` of contract {}",
        query.method,
        serde_json::to_string(&query.address).expect(zinc_const::panic::DATA_CONVERSION),
    );

    let contract = app_data
        .read()
        .expect(zinc_const::panic::SYNCHRONIZATION)
        .contracts
        .get(&query.address)
        .cloned()
        .ok_or_else(|| {
            Error::ContractNotFound(
                serde_json::to_string(&query.address).expect(zinc_const::panic::DATA_CONVERSION),
            )
        })?;
    let account_id = contract.account_id.ok_or_else(|| {
        Error::ContractLocked(
            serde_json::to_string(&query.address).expect(zinc_const::panic::DATA_CONVERSION),
        )
    })?;

    let method = match contract.build.methods.get(query.method.as_str()).cloned() {
        Some(method) => method,
        None => return Err(Error::MethodNotFound(query.method)),
    };
    if !method.is_mutable {
        return Err(Error::MethodIsImmutable(query.method));
    }

    log::debug!("Initializing the contract wallet");
    let provider = zksync::Provider::new(query.network);
    let wallet_credentials = zksync::WalletCredentials::from_eth_signer(
        query.address,
        PrivateKeySigner::new(contract.eth_private_key),
        query.network,
    )
    .await?;
    let wallet = zksync::Wallet::new(provider, wallet_credentials).await?;

    let input_value = BuildValue::try_from_typed_json(body.arguments, method.input)
        .map_err(Error::InvalidInput)?;

    log::debug!("Loading the pre-transaction contract storage");
    let database_fields = postgresql
        .select_fields(FieldSelectInput::new(account_id))
        .await?;
    let storage = Storage::new_with_data(
        database_fields,
        contract.build.storage.as_slice(),
        contract.eth_address,
        &wallet,
    )
    .await?;

    log::debug!("Running the contract method on the virtual machine");
    let method = query.method;
    let contract_build = contract.build;
    let vm_time = std::time::Instant::now();
    log::debug!("input_value:{:?}", input_value);
    let mut transaction_msgs: Vec<TransactionMsg> = Vec::new();

    for transaction in (&body.transaction).iter() {
        let transaction_msg = transaction.try_to_msg(&wallet)?;
        log::debug!("transactionMsg:{:?}", transaction_msg);
        transaction_msgs.push(transaction_msg);
    }

    let output = async_std::task::spawn_blocking(move || {
        zinc_vm::ContractFacade::new(contract_build).run::<Bn256>(ContractInput::new(
            input_value,
            storage.into_build(),
            method,
            transaction_msgs,
        ))
    })
    .await
    .map_err(Error::RuntimeError)?;
    log::debug!("VM executed in {} ms", vm_time.elapsed().as_millis());

    log::debug!("Loading the post-transaction contract storage");
    let storage = Storage::from_build(output.storage).into_database_update(account_id);

    log::debug!("Building the transaction list");
    let mut transactions = body.transaction;
    // if let ZkSyncTx::Transfer(ref transfer) = transactions[0].tx {
    //     log::debug!("transfer:{:?}", transfer);
    //     let token = wallet
    //         .tokens
    //         .resolve(transfer.token.into())
    //         .ok_or_else(|| Error::TokenNotFound(transfer.token.to_string()))?;

    //     log::debug!(
    //         "Sending {} {} from {} to {} with total batch fee {} {}",
    //         zksync_utils::format_units(&transfer.amount, token.decimals),
    //         token.symbol,
    //         serde_json::to_string(&transfer.from).expect(zinc_const::panic::DATA_CONVERSION),
    //         serde_json::to_string(&transfer.to).expect(zinc_const::panic::DATA_CONVERSION),
    //         zksync_utils::format_units(&transfer.fee, token.decimals),
    //         token.symbol,
    //     );
    // }

    let mut nonce = wallet
        .provider
        .account_info(query.address)
        .await?
        .committed
        .nonce;
    log::debug!("output:{:?}", output.transfers);
    for transfer in output.transfers.into_iter() {
        let recipient = transfer.recipient.into();
        let token = wallet
            .tokens
            .resolve(
                zinc_zksync::eth_address_from_vec(transfer.token_address.to_bytes_be().to_vec())
                    .into(),
            )
            .ok_or_else(|| {
                Error::TokenNotFound(
                    transfer
                        .token_address
                        .to_str_radix(zinc_const::base::HEXADECIMAL),
                )
            })?;
        let amount = zksync::utils::closest_packable_token_amount(
            &zinc_zksync::num_compat_backward(transfer.amount),
        );
        let fee = BigUint::zero();

        log::debug!(
            "Sending {} {} from {} to {}",
            zksync_utils::format_units(&amount, token.decimals),
            token.symbol,
            serde_json::to_string(&query.address).expect(zinc_const::panic::DATA_CONVERSION),
            serde_json::to_string(&recipient).expect(zinc_const::panic::DATA_CONVERSION),
        );

        let (transfer, signature) = wallet
            .signer
            .sign_transfer(token, amount, fee, recipient, nonce)
            .await?;
        transactions.push(Transaction::new(
            ZkSyncTx::Transfer(Box::new(transfer)),
            signature.expect(zinc_const::panic::VALUE_ALWAYS_EXISTS),
        ));

        nonce += 1;
    }

    log::debug!(
        "Sending the transactions to zkSync on network `{}`,transactions: {:?}",
        query.network,
        &transactions
    );
    let handles: Vec<SyncTransactionHandle> = wallet
        .provider
        .send_txs_batch(
            transactions
                .into_iter()
                .map(|transaction| {
                    (
                        transaction.tx,
                        Some(transaction.ethereum_signature.signature),
                    )
                })
                .collect(),
            None,
        )
        .await?
        .into_iter()
        .map(|tx_hash| {
            let mut handle = SyncTransactionHandle::new(tx_hash, wallet.provider.clone())
                .commit_timeout(Duration::from_secs(10));
            handle
                .polling_interval(Duration::from_millis(200))
                .expect("Validated inside the method");
            handle
        })
        .collect();

    if let Some(handle) = handles.last() {
        log::debug!("Waiting for the batch transaction to be committed");

        let tx_info = handle.wait_for_commit().await?;
        if !tx_info.success.unwrap_or_default() {
            return Err(Error::TransferFailure(
                tx_info
                    .fail_reason
                    .unwrap_or_else(|| "Unknown error".to_owned()),
            ));
        }
    }

    log::debug!("Committing the contract storage state to the database");
    postgresql.update_fields(storage).await?;

    let response = json!({
        "output": output.result.into_json(),
    });

    log::debug!("The call has been successfully executed");
    Ok(Response::new_with_data(StatusCode::OK, response))
}
