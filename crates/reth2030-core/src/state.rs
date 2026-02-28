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
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryState {
    accounts: BTreeMap<Address, Account>,
}

impl InMemoryState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_transaction(&mut self, tx: &Transaction) -> Result<(), StateError> {
        let sender = tx.from();
        let recipient = tx.to();
        let value = tx.value();

        if let Some(to) = recipient {
            self.transfer(sender, to, value)?;
        } else {
            self.debit_and_bump_nonce(sender, value)?;
        }

        Ok(())
    }

    pub fn apply_transactions(&mut self, txs: &[Transaction]) -> Result<(), StateError> {
        for tx in txs {
            self.apply_transaction(tx)?;
        }
        Ok(())
    }

    pub fn snapshot(&self) -> BTreeMap<Address, Account> {
        self.accounts.clone()
    }

    fn debit_and_bump_nonce(&mut self, address: Address, amount: u128) -> Result<(), StateError> {
        let account = self.accounts.entry(address).or_default();
        if account.balance < amount {
            return Err(StateError::InsufficientBalance {
                address,
                available: account.balance,
                requested: amount,
            });
        }
        account.balance -= amount;
        account.nonce = account.nonce.saturating_add(1);
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
