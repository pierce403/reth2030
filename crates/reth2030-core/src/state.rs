use reth2030_types::{Address, Transaction};
use std::collections::BTreeMap;

pub type StorageKey = [u8; 32];
pub type StorageValue = [u8; 32];

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Account {
    pub nonce: u64,
    pub balance: u128,
    pub code: Vec<u8>,
    pub storage: BTreeMap<StorageKey, StorageValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateError {
    InsufficientBalance {
        address: Address,
        available: u128,
        requested: u128,
    },
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateError::InsufficientBalance {
                address,
                available,
                requested,
            } => write!(
                f,
                "insufficient balance for {:?}: available={}, requested={}",
                address, available, requested
            ),
        }
    }
}

impl std::error::Error for StateError {}

pub trait StateStore {
    fn get_account(&self, address: &Address) -> Option<Account>;
    fn upsert_account(&mut self, address: Address, account: Account);
    fn get_storage(&self, address: &Address, key: &StorageKey) -> Option<StorageValue>;
    fn set_storage(&mut self, address: Address, key: StorageKey, value: StorageValue);
    fn transfer(&mut self, from: Address, to: Address, value: u128) -> Result<(), StateError>;

    fn apply_transaction(&mut self, tx: &Transaction) -> Result<(), StateError> {
        let from = tx.from();
        let value = tx.value();

        let mut sender = self.get_account(&from).unwrap_or_default();
        if sender.balance < value {
            return Err(StateError::InsufficientBalance {
                address: from,
                available: sender.balance,
                requested: value,
            });
        }
        sender.balance -= value;
        sender.nonce = sender.nonce.saturating_add(1);
        self.upsert_account(from, sender);

        if let Some(to) = tx.to() {
            let mut recipient = self.get_account(&to).unwrap_or_default();
            recipient.balance = recipient.balance.saturating_add(value);
            self.upsert_account(to, recipient);
        }

        Ok(())
    }

    fn apply_transactions(&mut self, txs: &[Transaction]) -> Result<(), StateError> {
        for tx in txs {
            self.apply_transaction(tx)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryState {
    accounts: BTreeMap<Address, Account>,
}

impl InMemoryState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> BTreeMap<Address, Account> {
        self.accounts.clone()
    }

    fn debit_and_bump_nonce(&mut self, address: Address, amount: u128) -> Result<(), StateError> {
        let mut account = self.accounts.get(&address).cloned().unwrap_or_default();
        if account.balance < amount {
            return Err(StateError::InsufficientBalance {
                address,
                available: account.balance,
                requested: amount,
            });
        }
        account.balance -= amount;
        account.nonce = account.nonce.saturating_add(1);
        self.accounts.insert(address, account);
        Ok(())
    }
}

impl StateStore for InMemoryState {
    fn get_account(&self, address: &Address) -> Option<Account> {
        self.accounts.get(address).cloned()
    }

    fn upsert_account(&mut self, address: Address, account: Account) {
        self.accounts.insert(address, account);
    }

    fn get_storage(&self, address: &Address, key: &StorageKey) -> Option<StorageValue> {
        self.accounts
            .get(address)
            .and_then(|account| account.storage.get(key).copied())
    }

    fn set_storage(&mut self, address: Address, key: StorageKey, value: StorageValue) {
        let account = self.accounts.entry(address).or_default();
        account.storage.insert(key, value);
    }

    fn transfer(&mut self, from: Address, to: Address, value: u128) -> Result<(), StateError> {
        self.debit_and_bump_nonce(from, value)?;

        let recipient = self.accounts.entry(to).or_default();
        recipient.balance = recipient.balance.saturating_add(value);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Account, InMemoryState, StateError, StateStore};
    use reth2030_types::{LegacyTx, Transaction};

    fn addr(byte: u8) -> [u8; 20] {
        [byte; 20]
    }

    #[test]
    fn storage_roundtrip_is_deterministic() {
        let mut state = InMemoryState::new();
        let key = [0x11; 32];
        let value = [0x22; 32];
        state.set_storage(addr(0x01), key, value);

        assert_eq!(state.get_storage(&addr(0x01), &key), Some(value));
    }

    #[test]
    fn set_storage_creates_account_with_default_fields() {
        let mut state = InMemoryState::new();
        let key = [0x11; 32];
        let value = [0x22; 32];

        state.set_storage(addr(0x01), key, value);

        let account = state
            .get_account(&addr(0x01))
            .expect("storage write creates account");
        assert_eq!(account.nonce, 0);
        assert_eq!(account.balance, 0);
        assert!(account.code.is_empty());
        assert_eq!(account.storage.get(&key), Some(&value));
    }

    #[test]
    fn set_storage_overwrites_per_account_without_leakage() {
        let mut state = InMemoryState::new();
        let key = [0x11; 32];
        let first = [0x22; 32];
        let second = [0x33; 32];
        let other = [0x44; 32];

        state.set_storage(addr(0x01), key, first);
        state.set_storage(addr(0x02), key, other);
        state.set_storage(addr(0x01), key, second);

        assert_eq!(state.get_storage(&addr(0x01), &key), Some(second));
        assert_eq!(state.get_storage(&addr(0x02), &key), Some(other));
    }

    #[test]
    fn set_storage_preserves_existing_account_fields_and_other_keys() {
        let mut state = InMemoryState::new();
        let preserved_key = [0x09; 32];
        let preserved_value = [0x0a; 32];
        let target_key = [0x11; 32];
        let target_value = [0x22; 32];

        let mut storage = std::collections::BTreeMap::new();
        storage.insert(preserved_key, preserved_value);
        state.upsert_account(
            addr(0x01),
            Account {
                nonce: 9,
                balance: 777,
                code: vec![0xde, 0xad, 0xbe, 0xef],
                storage,
            },
        );

        state.set_storage(addr(0x01), target_key, target_value);

        let account = state
            .get_account(&addr(0x01))
            .expect("account must remain present");
        assert_eq!(account.nonce, 9);
        assert_eq!(account.balance, 777);
        assert_eq!(account.code, vec![0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(account.storage.get(&preserved_key), Some(&preserved_value));
        assert_eq!(account.storage.get(&target_key), Some(&target_value));
    }

    #[test]
    fn transfer_updates_balances_and_nonce() {
        let mut state = InMemoryState::new();
        state.upsert_account(
            addr(0xaa),
            Account {
                balance: 50,
                ..Account::default()
            },
        );

        state
            .transfer(addr(0xaa), addr(0xbb), 20)
            .expect("transfer");

        let from = state.get_account(&addr(0xaa)).expect("sender account");
        let to = state.get_account(&addr(0xbb)).expect("recipient account");
        assert_eq!(from.balance, 30);
        assert_eq!(from.nonce, 1);
        assert_eq!(to.balance, 20);
    }

    #[test]
    fn transfer_errors_when_balance_is_insufficient() {
        let mut state = InMemoryState::new();
        state.upsert_account(
            addr(0xaa),
            Account {
                balance: 5,
                ..Account::default()
            },
        );

        let err = state
            .transfer(addr(0xaa), addr(0xbb), 6)
            .expect_err("must fail");
        assert_eq!(
            err,
            StateError::InsufficientBalance {
                address: addr(0xaa),
                available: 5,
                requested: 6,
            }
        );
    }

    #[test]
    fn transfer_error_is_atomic_for_sender_and_recipient() {
        let mut state = InMemoryState::new();
        state.upsert_account(
            addr(0xaa),
            Account {
                nonce: 7,
                balance: 5,
                ..Account::default()
            },
        );
        let before = state.snapshot();

        let err = state
            .transfer(addr(0xaa), addr(0xbb), 6)
            .expect_err("must fail");
        assert_eq!(
            err,
            StateError::InsufficientBalance {
                address: addr(0xaa),
                available: 5,
                requested: 6,
            }
        );
        assert_eq!(state.snapshot(), before);
        assert_eq!(state.get_account(&addr(0xbb)), None);
    }

    #[test]
    fn transfer_from_missing_sender_is_atomic() {
        let mut state = InMemoryState::new();
        let before = state.snapshot();

        let err = state
            .transfer(addr(0xaa), addr(0xbb), 1)
            .expect_err("must fail");
        assert_eq!(
            err,
            StateError::InsufficientBalance {
                address: addr(0xaa),
                available: 0,
                requested: 1,
            }
        );
        assert_eq!(state.snapshot(), before);
        assert_eq!(state.get_account(&addr(0xaa)), None);
        assert_eq!(state.get_account(&addr(0xbb)), None);
    }

    #[test]
    fn zero_value_transfer_from_missing_sender_creates_accounts_deterministically() {
        let mut state = InMemoryState::new();

        state
            .transfer(addr(0xaa), addr(0xbb), 0)
            .expect("zero-value transfer");

        let from = state.get_account(&addr(0xaa)).expect("sender account");
        let to = state.get_account(&addr(0xbb)).expect("recipient account");
        assert_eq!(from.balance, 0);
        assert_eq!(from.nonce, 1);
        assert_eq!(to.balance, 0);
        assert_eq!(to.nonce, 0);
    }

    #[test]
    fn transfer_saturates_recipient_balance() {
        let mut state = InMemoryState::new();
        state.upsert_account(
            addr(0xaa),
            Account {
                balance: 10,
                ..Account::default()
            },
        );
        state.upsert_account(
            addr(0xbb),
            Account {
                balance: u128::MAX - 2,
                ..Account::default()
            },
        );

        state
            .transfer(addr(0xaa), addr(0xbb), 5)
            .expect("transfer succeeds");

        let sender = state.get_account(&addr(0xaa)).expect("sender account");
        let recipient = state.get_account(&addr(0xbb)).expect("recipient account");
        assert_eq!(sender.balance, 5);
        assert_eq!(sender.nonce, 1);
        assert_eq!(recipient.balance, u128::MAX);
        assert_eq!(recipient.nonce, 0);
    }

    #[test]
    fn transfer_to_self_preserves_balance_and_bumps_nonce() {
        let mut state = InMemoryState::new();
        state.upsert_account(
            addr(0xaa),
            Account {
                balance: 42,
                ..Account::default()
            },
        );

        state
            .transfer(addr(0xaa), addr(0xaa), 7)
            .expect("self transfer");

        let account = state.get_account(&addr(0xaa)).expect("account exists");
        assert_eq!(account.balance, 42);
        assert_eq!(account.nonce, 1);
    }

    #[test]
    fn apply_transactions_is_deterministic() {
        let tx1 = Transaction::Legacy(LegacyTx {
            nonce: 0,
            from: addr(0x01),
            to: Some(addr(0x02)),
            gas_limit: 21_000,
            gas_price: 1,
            value: 10,
            data: Vec::new(),
        });
        let tx2 = Transaction::Legacy(LegacyTx {
            nonce: 1,
            from: addr(0x01),
            to: Some(addr(0x03)),
            gas_limit: 21_000,
            gas_price: 1,
            value: 5,
            data: Vec::new(),
        });

        let mut state_a = InMemoryState::new();
        state_a.upsert_account(
            addr(0x01),
            Account {
                balance: 30,
                ..Account::default()
            },
        );

        let mut state_b = state_a.clone();
        state_a
            .apply_transactions(&[tx1.clone(), tx2.clone()])
            .expect("first run");
        state_b.apply_transactions(&[tx1, tx2]).expect("second run");

        assert_eq!(state_a.snapshot(), state_b.snapshot());
    }
}
