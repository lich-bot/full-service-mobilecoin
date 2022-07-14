// Copyright (c) 2020-2021 MobileCoin Inc.

//! Service for managing Txos.

use crate::{
    db::{
        account::AccountID,
        assigned_subaddress::AssignedSubaddressModel,
        models::{AssignedSubaddress, Txo},
        txo::{TxoID, TxoModel, TxoStatus},
        WalletDbError,
    },
    service::transaction::{TransactionService, TransactionServiceError},
    WalletService,
};
use displaydoc::Display;
use mc_connection::{BlockchainConnection, UserTxConnection};
use mc_fog_report_validation::FogPubkeyResolver;
use mc_mobilecoind::payments::TxProposal;

/// Errors for the Txo Service.
#[derive(Display, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum TxoServiceError {
    /// Error interacting with the database: {0}
    Database(WalletDbError),

    /// Error with LedgerDB: {0}
    LedgerDB(mc_ledger_db::Error),

    /// Diesel Error: {0}
    Diesel(diesel::result::Error),

    /// Minted Txo should contain confirmation: {0}
    MissingConfirmation(String),

    /// Error with the Transaction Service: {0}
    TransactionService(TransactionServiceError),

    /// No account found to spend this txo
    TxoNotSpendableByAnyAccount(String),

    /// Txo Not Spendable
    TxoNotSpendable(String),
}

impl From<WalletDbError> for TxoServiceError {
    fn from(src: WalletDbError) -> Self {
        Self::Database(src)
    }
}

impl From<mc_ledger_db::Error> for TxoServiceError {
    fn from(src: mc_ledger_db::Error) -> Self {
        Self::LedgerDB(src)
    }
}

impl From<diesel::result::Error> for TxoServiceError {
    fn from(src: diesel::result::Error) -> Self {
        Self::Diesel(src)
    }
}

impl From<TransactionServiceError> for TxoServiceError {
    fn from(src: TransactionServiceError) -> Self {
        Self::TransactionService(src)
    }
}

/// Trait defining the ways in which the wallet can interact with and manage
/// Txos.
pub trait TxoService {
    /// List the Txos for a given account in the wallet.
    fn list_txos(
        &self,
        account_id: &AccountID,
        status: Option<TxoStatus>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<(Txo, TxoStatus)>, TxoServiceError>;

    /// Get a Txo from the wallet.
    fn get_txo(&self, txo_id: &TxoID) -> Result<(Txo, TxoStatus), TxoServiceError>;

    /// Split a Txo
    fn split_txo(
        &self,
        txo_id: &TxoID,
        output_values: &[String],
        subaddress_index: Option<i64>,
        fee: Option<String>,
        tombstone_block: Option<String>,
    ) -> Result<TxProposal, TxoServiceError>;

    /// List the Txos for a given address for an account in the wallet.
    fn get_all_txos_for_address(
        &self,
        address: &str,
    ) -> Result<Vec<(Txo, TxoStatus)>, TxoServiceError>;
}

impl<T, FPR> TxoService for WalletService<T, FPR>
where
    T: BlockchainConnection + UserTxConnection + 'static,
    FPR: FogPubkeyResolver + Send + Sync + 'static,
{
    fn list_txos(
        &self,
        account_id: &AccountID,
        status: Option<TxoStatus>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<(Txo, TxoStatus)>, TxoServiceError> {
        let conn = &self.wallet_db.get_conn()?;

        let txos_and_statuses = Txo::list_for_account(
            &account_id.to_string(),
            status,
            limit,
            offset,
            Some(0),
            conn,
        )?
        .into_iter()
        .map(|txo| {
            let status = txo.status(conn)?;
            Ok((txo, status))
        })
        .collect::<Result<Vec<(Txo, TxoStatus)>, WalletDbError>>()?;

        Ok(txos_and_statuses)
    }

    fn get_txo(&self, txo_id: &TxoID) -> Result<(Txo, TxoStatus), TxoServiceError> {
        let conn = self.wallet_db.get_conn()?;
        let txo = Txo::get(&txo_id.to_string(), &conn)?;
        let status = txo.status(&conn)?;
        Ok((txo, status))
    }

    fn split_txo(
        &self,
        txo_id: &TxoID,
        output_values: &[String],
        subaddress_index: Option<i64>,
        fee: Option<String>,
        tombstone_block: Option<String>,
    ) -> Result<TxProposal, TxoServiceError> {
        use crate::service::txo::TxoServiceError::TxoNotSpendableByAnyAccount;

        let conn = self.wallet_db.get_conn()?;
        let txo_details = Txo::get(&txo_id.to_string(), &conn)?;

        let account_id_hex = txo_details
            .account_id
            .ok_or(TxoNotSpendableByAnyAccount(txo_details.id))?;

        let address_to_split_into: AssignedSubaddress =
            AssignedSubaddress::get_for_account_by_index(
                &account_id_hex,
                subaddress_index.unwrap_or(0),
                &conn,
            )?;

        let mut addresses_and_values = Vec::new();
        for output_value in output_values.iter() {
            addresses_and_values.push((
                address_to_split_into.assigned_subaddress_b58.clone(),
                output_value.to_string(),
            ))
        }

        Ok(self.build_transaction(
            &account_id_hex,
            &addresses_and_values,
            Some(&[txo_id.to_string()].to_vec()),
            fee,
            tombstone_block,
            None,
            None,
        )?)
    }

    fn get_all_txos_for_address(
        &self,
        address: &str,
    ) -> Result<Vec<(Txo, TxoStatus)>, TxoServiceError> {
        let conn = &self.wallet_db.get_conn()?;
        let txos = Txo::list_for_address(address, None, None, None, Some(0), conn)?;

        let txos_and_statuses = txos
            .into_iter()
            .map(|txo| {
                let status = txo.status(conn)?;
                Ok((txo, status))
            })
            .collect::<Result<Vec<(Txo, TxoStatus)>, WalletDbError>>()?;
        Ok(txos_and_statuses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        service::{
            account::AccountService, balance::BalanceService, transaction::TransactionService,
        },
        test_utils::{
            add_block_to_ledger_db, get_test_ledger, manually_sync_account, setup_wallet_service,
            MOB,
        },
        util::b58::b58_encode_public_address,
    };
    use mc_account_keys::{AccountKey, PublicAddress};
    use mc_common::logger::{test_with_logger, Logger};
    use mc_crypto_rand::RngCore;
    use mc_transaction_core::{ring_signature::KeyImage, tokens::Mob, Token};
    use rand::{rngs::StdRng, SeedableRng};

    #[test_with_logger]
    fn test_txo_lifecycle(logger: Logger) {
        let mut rng: StdRng = SeedableRng::from_seed([20u8; 32]);

        let known_recipients: Vec<PublicAddress> = Vec::new();
        let mut ledger_db = get_test_ledger(5, &known_recipients, 12, &mut rng);

        let service = setup_wallet_service(ledger_db.clone(), logger.clone());
        let alice = service
            .create_account(
                Some("Alice's Main Account".to_string()),
                "".to_string(),
                "".to_string(),
                "".to_string(),
            )
            .unwrap();

        // Add a block with a transaction for this recipient
        // Add a block with a txo for this address
        let alice_account_key: AccountKey = mc_util_serial::decode(&alice.account_key).unwrap();
        let alice_account_id = AccountID::from(&alice_account_key);
        let alice_public_address = alice_account_key.subaddress(alice.main_subaddress_index as u64);
        add_block_to_ledger_db(
            &mut ledger_db,
            &vec![alice_public_address.clone()],
            100 * MOB,
            &vec![KeyImage::from(rng.next_u64())],
            &mut rng,
        );

        manually_sync_account(&ledger_db, &service.wallet_db, &alice_account_id, &logger);

        // Verify balance for Alice
        let balance = service.get_balance_for_account(&alice_account_id).unwrap();
        let balance_pmob = balance.get(&Mob::ID).unwrap();

        assert_eq!(balance_pmob.unspent, 100 * MOB as u128);

        // Verify that we have 1 txo
        let txos = service
            .list_txos(&alice_account_id, None, None, None)
            .unwrap();
        assert_eq!(txos.len(), 1);

        // Add another account
        let bob = service
            .create_account(
                Some("Bob's Main Account".to_string()),
                "".to_string(),
                "".to_string(),
                "".to_string(),
            )
            .unwrap();

        // Construct a new transaction to Bob
        let bob_account_key: AccountKey = mc_util_serial::decode(&bob.account_key).unwrap();
        let tx_proposal = service
            .build_transaction(
                &alice.id,
                &vec![(
                    b58_encode_public_address(
                        &bob_account_key.subaddress(bob.main_subaddress_index as u64),
                    )
                    .unwrap(),
                    "42000000000000".to_string(),
                )],
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();
        let _submitted = service
            .submit_transaction(tx_proposal, None, Some(alice.id.clone()))
            .unwrap();

        let pending: Vec<(Txo, TxoStatus)> = service
            .list_txos(
                &AccountID(alice.id.clone()),
                Some(TxoStatus::Pending),
                None,
                None,
            )
            .unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].0.value, 100000000000000);

        // Our balance should reflect the various statuses of our txos
        let balance = service
            .get_balance_for_account(&AccountID(alice.id))
            .unwrap();
        let balance_pmob = balance.get(&Mob::ID).unwrap();

        assert_eq!(balance_pmob.unverified, 0);
        assert_eq!(balance_pmob.unspent, 0);
        assert_eq!(balance_pmob.pending, 100 * MOB as u128);
        assert_eq!(balance_pmob.spent, 0);
        assert_eq!(balance_pmob.orphaned, 0);
    }
}
