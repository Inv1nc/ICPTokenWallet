use candid::{CandidType, Decode, Deserialize, Encode, Nat, Principal};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, DefaultMemoryImpl, StableBTreeMap, Storable};
use icrc_ledger_types::icrc1::account::Account;
use icrc_ledger_types::icrc1::transfer::{BlockIndex, TransferArg, TransferError};
use icrc_ledger_types::icrc2::transfer_from::{TransferFromArgs, TransferFromError};
use std::{borrow::Cow, cell::RefCell};

// Virtual memory alias
type Memory = VirtualMemory<DefaultMemoryImpl>;

const PRINCIPAL_SIZE: u32 = 500;

// Struct wrapping Principal for StableBTreeMap
#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PrincipalKey(Principal);

// Storable Implementation for PrincipalKey
impl Storable for PrincipalKey {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// BoundedStorable for PrincipalKey
impl BoundedStorable for PrincipalKey {
    const MAX_SIZE: u32 = PRINCIPAL_SIZE;
    const IS_FIXED_SIZE: bool = false;
}

// Thread-local storage for memory manager and balances
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static BALANCE_MAP: RefCell<StableBTreeMap<PrincipalKey, u64, Memory>> = RefCell::new(StableBTreeMap::init(
        MEMORY_MANAGER.with(|m|m.borrow().get(MemoryId::new(0)))
    ));
}

// Query function to get caller's token balance
#[ic_cdk::query]
async fn get_token_balance() -> u64 {
    let caller_key = PrincipalKey(ic_cdk::caller());
    BALANCE_MAP.with(|tw| tw.borrow().get(&caller_key).unwrap_or(0)) // Return balance or 0
}

// Update function to deposit tokens
#[ic_cdk::update]
async fn deposit_tokens(amount: u64) -> Result<BlockIndex, String> {
    // Create transfer_from arguments
    let transfer_from_args: TransferFromArgs = TransferFromArgs {
        from: Account::from(ic_cdk::caller()),
        memo: None,
        amount: Nat::from(amount),
        spender_subaccount: None,
        fee: None,
        to: Account::from(ic_cdk::id()),
        created_at_time: None,
    };

    // Calling ledger for transfer
    let transfer_result =
        ic_cdk::call::<(TransferFromArgs,), (Result<BlockIndex, TransferFromError>,)>(
            Principal::from_text("mxzaz-hqaaa-aaaar-qaada-cai")
                .expect("Could not decode the principal."),
            "icrc2_transfer_from",
            (transfer_from_args,),
        )
        .await;

    // Handle result
    let block_index = match transfer_result {
        Ok((Ok(index),)) => index,
        Ok((Err(e),)) => return Err(format!("Ledger transfer error: {:?}", e)),
        Err(e) => return Err(format!("Failed to call the ledger: {:?}", e)),
    };

    // Update caller's balance
    let caller_key = PrincipalKey(ic_cdk::caller());

    BALANCE_MAP.with(|tw| {
        let current_balance = tw.borrow().get(&caller_key).unwrap_or(0);
        let updated_balance = current_balance + amount;
        tw.borrow_mut().insert(caller_key, updated_balance);
    });

    Ok(block_index) // Return block index
}

// Update function to send tokens to another user
#[ic_cdk::update]
async fn send_tokens(amount: u64, to: Principal) -> Result<BlockIndex, String> {
    let caller_key = PrincipalKey(ic_cdk::caller());
    let current_balance = BALANCE_MAP.with(|tw| tw.borrow().get(&caller_key).unwrap_or(0));

    // Check if enough balance exists
    if current_balance < amount {
        return Err("Insufficient balance".to_string());
    }

    ic_cdk::println!("Transfering {} tokens to account {}", &amount, &to);

    // Create transfer arguments for the ledger
    let transfer_args: TransferArg = TransferArg {
        memo: None,
        amount: Nat::from(amount),
        from_subaccount: None,
        fee: None,
        to: Account::from(to),
        created_at_time: None,
    };

    // Call ledger for transfer
    let transfer_result = ic_cdk::call::<(TransferArg,), (Result<BlockIndex, TransferError>,)>(
        Principal::from_text("mxzaz-hqaaa-aaaar-qaada-cai")
            .expect("Could not decode the principal."),
        "icrc1_transfer",
        (transfer_args,),
    )
    .await;

    // Handle result
    let block_index = match transfer_result {
        Ok((Ok(index),)) => index,
        Ok((Err(e),)) => return Err(format!("Ledger transfer error: {:?}", e)),
        Err(e) => return Err(format!("Failed to call the ledger: {:?}", e)),
    };

    // Update caller's balance
    let caller_key = PrincipalKey(ic_cdk::caller());

    BALANCE_MAP.with(|tw| {
        let current_balance = tw.borrow().get(&caller_key).unwrap_or(0);
        let updated_balance = current_balance - amount;
        tw.borrow_mut().insert(caller_key, updated_balance);
    });

    Ok(block_index) // Return block index
}
