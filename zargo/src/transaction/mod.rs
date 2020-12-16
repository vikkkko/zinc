//!
//! The transaction tools.
//!

pub mod error;

use num::BigUint;

use zksync::web3::types::Address;
use zksync_eth_signer::PrivateKeySigner;
use zksync_types::tx::ZkSyncTx;
use zksync_types::TokenLike;
use zksync_types::TxFeeTypes;

use zinc_zksync::TransactionMsg;

use self::error::Error;

///
/// Initializes a new initial zero transfer to assign an account ID to a newly created contract.
///
pub async fn new_initial(
    wallet: &zksync::Wallet<PrivateKeySigner>,
    recipient: Address,
    token_symbol: String,
    amount: BigUint,
) -> Result<zinc_zksync::Transaction, Error> {
    let token_like = TokenLike::Symbol(token_symbol);
    let token = wallet
        .tokens
        .resolve(token_like.clone())
        .ok_or(Error::TokenNotFound)?;

    let amount =
        zksync::utils::closest_packable_token_amount(&zinc_zksync::num_compat_backward(amount));
    let fee = wallet
        .provider
        .get_tx_fee(TxFeeTypes::Transfer, recipient, token_like)
        .await
        .map_err(Error::FeeGetting)?
        .total_fee;
    let nonce = wallet
        .provider
        .account_info(wallet.signer.address)
        .await
        .map_err(Error::AccountInfoRetrieving)?
        .committed
        .nonce;

    let (transfer, signature) = wallet
        .signer
        .sign_transfer(token, amount, fee, recipient, nonce)
        .await
        .map_err(Error::TransactionSigning)?;
    let signature = signature.expect(zinc_const::panic::DATA_CONVERSION);

    Ok(zinc_zksync::Transaction::new(
        ZkSyncTx::Transfer(Box::new(transfer)),
        signature,
    ))
}

///
/// Converts an array of input transfers into an array of signed zkSync transactions.
///
pub async fn try_into_zksync(
    transaction: TransactionMsg,
    wallet: &zksync::Wallet<PrivateKeySigner>,
    contract_fee: Option<BigUint>,
    nonce_adjust: u32,
) -> Result<zinc_zksync::Transaction, Error> {
    let token = wallet
        .tokens
        .resolve(transaction.token_address.into())
        .ok_or(Error::TokenNotFound)?;
    let amount = zksync::utils::closest_packable_token_amount(&transaction.amount);
    let fee = wallet
        .provider
        .get_tx_fee(
            TxFeeTypes::Transfer,
            wallet.signer.address,
            transaction.token_address,
        )
        .await
        .map_err(Error::FeeGetting)?
        .total_fee
        + contract_fee
            .map(zinc_zksync::num_compat_backward)
            .unwrap_or_default();
    let fee = zksync::utils::closest_packable_fee_amount(&fee);
    let nonce = wallet
        .provider
        .account_info(wallet.signer.address)
        .await
        .map_err(Error::AccountInfoRetrieving)?
        .committed
        .nonce;

    let (transfer, signature) = wallet
        .signer
        .sign_transfer(
            token,
            amount,
            fee,
            transaction.recipient,
            nonce + nonce_adjust,
        )
        .await
        .map_err(Error::TransactionSigning)?;
    let signature = signature.expect(zinc_const::panic::DATA_CONVERSION);

    Ok(zinc_zksync::Transaction::new(
        ZkSyncTx::Transfer(Box::new(transfer)),
        signature,
    ))
}
