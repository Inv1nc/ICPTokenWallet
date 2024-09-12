# Token Wallet with ICRC-1 and ICRC-2 Ledger

This project implements a **Token Wallet** that securely holds tokens on behalf of users.

The wallet interacts with the Internet Computer's ICRC-1 and ICRC-2 token standards, allowing users to deposit, hold, and transfer tokens.

#### Key Features:
1. **Deposit Tokens**:
   - Users must first approve the wallet to transfer tokens on their behalf.
   - Once approved, users can deposit tokens by calling the **`deposit_tokens`** function and specifying the amount they want to deposit.
   - The deposited tokens are then held within the wallet contract, and the user's balance is updated accordingly.

2. **Hold Tokens**:
   - The wallet securely holds tokens in the user's account within the contract. The balances are mapped to each user's **Principal ID**.
   - Users can check their wallet balance at any time by calling the **`get_token_balance`** function.

3. **Transfer Tokens**:
   - Users can transfer tokens to another account by calling the **`send_tokens`** function.
   - To withdraw tokens back to their personal account, users simply call **`send_tokens`**, passing their own **Principal ID** and the amount they wish to withdraw.
   - The wallet verifies that the user has enough tokens before executing the transfer.

#### Workflow Overview:
1. **Token Deposit**:
   - User approves the wallet contract to transfer tokens on their behalf.
   - User calls the **`deposit_tokens`** function with the desired amount, which transfers tokens to the wallet.

2. **Check Balance**:
   - User calls **`get_token_balance`** to view the current token balance held in their wallet.

3. **Token Withdrawal or Transfer**:
   - To withdraw tokens, the user calls **`send_tokens`**, passing their own **Principal ID** and the amount they want to withdraw.
   - To transfer tokens to another user, the user calls **`send_tokens`** with the recipient’s **Principal ID** and the amount to transfer.

This wallet ensures the security and management of tokens, allowing users to deposit, hold, and withdraw their tokens efficiently and securely.

**Attention:** This contract assumes that the token transfer fee is set to `0`.

---

## Prerequisites

Before getting started, ensure that the following are installed:
- [DFINITY SDK (DFX)](https://internetcomputer.org/docs/current/developer-docs/getting-started/install/)
- Rust programming language and necessary libraries for building Rust-based canisters.

## Project Structure

This project includes two main functionalities:
1. **Token Balance Management** using a Stable BTree Map.
2. **Ledger Interactions** for deposits and transfers of tokens.

The core components are:
- **PrincipalKey:** A structure that wraps around the `Principal` of the caller, which acts as the unique identifier for each user in the system.
- **Stable Memory Management:** The balances are stored using a Stable BTree Map, backed by stable memory provided by the Internet Computer.

---

## Steps for Deployment

### Step 1: Start the Local Replica

Ensure the Internet Computer is running in the background:

```bash
dfx start --background --clean
```

### Step 2: Create a New Identity for Minting

Create a new identity `(minter)` for minting tokens and set it as the active identity:

```bash
dfx identity new minter
dfx identity use minter
export MINTER=$(dfx identity get-principal)
```

Transfers from the minting account will create Mint transactions, and transfers to the minting account will create Burn transactions.

### Step 3: Switch Back to the Default Identity

Switch back to your default identity and record its principal. This principal will be used to mint the initial balance during deployment:

```
dfx identity use default
export DEFAULT=$(dfx identity get-principal)
```

### Step 4: Deploy the ICRC-1 Ledger

Now you are ready to deploy the ICRC-1 ledger. This step sets the minting account, mints 100 tokens to the default principal, and sets a transfer fee:

```
dfx deploy icrc1_ledger_canister --argument "(variant { Init =
record {
     token_symbol = \"ICRC1\";
     token_name = \"L-ICRC1\";
     minting_account = record { owner = principal \"${MINTER}\" };
     transfer_fee = 0;
     metadata = vec {};
     initial_balances = vec { record { record { owner = principal \"${DEFAULT}\"; }; 10_000_000_000; }; };
     archive_options = record {
         num_blocks_to_archive = 1000;
         trigger_threshold = 2000;
         controller_id = principal \"${MINTER}\";
     };
	 feature_flags = opt record {
      icrc2 = true;
    };
 }
})"
```

### Step 5: Verify Ledger Deployment

You can verify that the ledger is working by checking the token balance for the `DEFAULT` account:

```
dfx canister call icrc1_ledger_canister icrc1_balance_of "(record {
  owner = principal \"${DEFAULT}\";
})"
```

The balance should show the tokens minted in the previous step.

### Step 6: Deploy the Token Wallet Canister

Deploy the `token_wallet_backend` canister that manages the token balances and interacts with the ledger:

```bash
dfx deploy token_wallet_backend
```

### Step 7: Approve Token Wallet Canister to Spend Tokens

Before the backend canister can interact with the ledger on behalf of a user, the user needs to approve the backend canister to spend tokens on their behalf:

```bash
dfx canister call --identity default icrc1_ledger_canister icrc2_approve "(
  record {
    spender= record {
      owner = principal \"$(dfx canister id token_wallet_backend)\";
    };
    amount = 1_000_000: nat;
  }
)"
```

### Step 8: Test Token Wallet Functions

You can interact with the token wallet backend by calling the following functions:

#### Check Token Balance:

This call will check the balance of the tokens in the wallet:

```bash
dfx canister call token_wallet_backend get_token_balance
```

#### Deposit Tokens:

You can deposit tokens into your wallet with the following command (e.g., 100 tokens):

```bash
dfx canister call token_wallet_backend deposit_tokens '(100)'
```

### Send Tokens:

You can also send tokens from your wallet to another principal. You can modify the wallet functions to send tokens from one account to another by adjusting the transfer function logic.

```
dfx canister call token_wallet_backend send_tokens "(50, principal \"<recipient-principal-id>\")"
```

To check the balance of a specific principal, use the following command:

```
dfx canister call icrc1_ledger_canister icrc1_balance_of "(record {
  owner = principal \"<principal-id>\";
})"
```

---

## Interacting with the Token Wallet

### 1. **Get Token Balance**

This query function checks the current token balance of the calling user.

```bash
dfx canister call token_wallet_backend get_token_balance
```

The balance is stored in the stable memory (`BALANCE_MAP`) and associated with the `Principal` of the user. If no balance exists for the caller, it returns `0`.

### 2. **Deposit Tokens**

This function allows users to deposit tokens into the wallet backend from their own account. The deposited tokens are recorded in the stable memory.

```bash
dfx canister call token_wallet_backend deposit_tokens '(100)'
```

#### Behind the Scenes:

- A `TransferFromArgs` struct is created to specify the amount to transfer, the sender (user), and the recipient (backend canister).
- The canister makes an inter-canister call to the ICRC-2 ledger's `icrc2_transfer_from` method.
- On success, the user's balance in `BALANCE_MAP` is updated by adding the deposited tokens.

### 3. **Send Tokens**

This function allows a user to send tokens from their balance in the wallet backend to another account. The recipient's `Principal` must be specified in the call.

```bash
dfx canister call token_wallet_backend send_tokens '(50, "<recipient-principal-id>")'
```

#### Behind the Scenes:

- The backend checks if the caller has sufficient tokens by querying the `BALANCE_MAP`.
- A `TransferArg` struct is created for transferring tokens via the ICRC-1 ledger's `icrc1_transfer` method.
- On success, the tokens are transferred to the recipient's account, and the caller's balance is updated in the `BALANCE_MAP`.

---

## Stable Memory and Balance Map

- **Stable Memory**: The project utilizes the `ic_stable_structures` crate to maintain a persistent BTree map in stable memory. This map is capable of storing token balances across upgrades and can grow as required.
- **Memory Management**: The `MemoryManager` struct initializes the stable memory, which is later used by the `BALANCE_MAP` to persist user balances. Memory management is handled through a memory ID (`MemoryId::new(0)`), which tracks the memory segments.

---

## Security Measures

The contract takes several steps to ensure the safety and security of the token wallet operations:

### 1. **Caller Verification**

- The contract identifies the caller using `ic_cdk::caller()`. This ensures that actions such as checking balances, depositing tokens, and transferring tokens are securely tied to the user's principal. No user can access or manipulate another user's balance.

### 2. **Balance Check for Transfers**

- Before transferring tokens, the contract verifies that the sender has sufficient funds to cover the transfer. This is checked by querying the current balance stored in the `BALANCE_MAP`. If the balance is insufficient, the transfer is denied with an error message:

  ```rust
  if current_balance < amount {
      return Err("Insufficient balance".to_string());
  }
  ```

  This ensures that users cannot send more tokens than they own.

### 3. **Ledger Call Error Handling**

- All interactions with the ICRC-1 and ICRC-2 ledgers are wrapped with comprehensive error handling. If any part of the ledger transfer fails (e.g., insufficient funds in the ledger, network issues), the contract returns a detailed error message explaining the issue:

  ```rust
  let block_index = match transfer_result {
      Ok((Ok(index),)) => index,
      Ok((Err(e),)) => return Err(format!("Ledger transfer error: {:?}", e)),
      Err(e) => return Err(format!("Failed to call the ledger: {:?}", e)),
  };
  ```

  This prevents inconsistent states or lost tokens during inter-canister communication.

### 4. **Immutable PrincipalKey**

- The `PrincipalKey` struct, which identifies users, is derived from the `Principal` of the caller. Since this `PrincipalKey` is used as the key in the `BALANCE_MAP`, it ensures that only the legitimate owner of a balance can query or update their balance. The key is also serialized and deserialized securely using the `Storable` and `BoundedStorable` traits to protect the integrity of the key across different memory accesses.

### 5. **Secure Memory Access**

- All balance storage and retrieval operations are wrapped in the thread-local `BALANCE_MAP` using `RefCell`. This ensures thread-safe access to the stable memory across different canister functions. This design prevents race conditions and guarantees that the balance map remains in a consistent state even when accessed concurrently.

### 6. **Fixed Token Amounts in Calls**

- The contract requires specific token amounts for deposits and transfers, avoiding ambiguous or non-deterministic amounts. The token amount is always converted to the `Nat` type, ensuring precision during ledger transfers.

### 7. **Access Control with Principals**

- The contract design tightly couples all token-related operations with the caller’s `Principal`. This ensures that only the authorized user (i.e., the one who originally deposited tokens) can transfer or check their token balance.

---

## References

- [Creating a token](https://internetcomputer.org/docs/current/developer-docs/defi/tokens/create)
- [Dfinity Examples](https://github.com/dfinity/examples)
