use std::collections::HashMap;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

const UNIQUE_SUPPLY: u64 = 1000000;

struct Transaction {
    sender_public_key: VerifyingKey,
    receiver_name: String,
    amount: u64,
    // Only private key-holder can produce signature, anyone with the public key can verify it,
    // proving authenticity and authorization of the transaction.
    signature: Signature,
}


// Global Ledger
struct State {
    // Map public key (Authority) -> Balance
    balances: HashMap<[u8; 32], u64> // Every key must be a 32-byte array (Ed25519 public key size)
}

impl State {
    fn new(god_public_key: VerifyingKey) -> Self {
        let mut balances = HashMap::new();
        // Rule 1 - All coins exists at birth in GOD
        balances.insert(god_public_key.to_bytes(), UNIQUE_SUPPLY);


        State { balances }
    }

    fn verify_and_apply_transaction(&mut self, tx: Transaction) -> Result<(), String> {
        // Create message that was signed (amount + receiver)
        // FRAGILE, ambiguity, amount = 12, reciever = 3 -> "123" as amount = 1, receiver = 23 -> "123"
        // We need canonicalization, or better yet, a structured message format (e.g. JSON, protobuf)
        let message = format!("{}{}", tx.amount, tx.receiver_name);

        // Also receiver should have same addressing system as the ledger.
        // Receiver should be VerifyingKey or at least a hash of it, not a free-form string.

        // Cryptographic Check: Did sender sign this?
        tx.sender_public_key
            .verify(message.as_bytes(), &tx.signature)
            .map_err(|_| "Invalid signature".to_string())?;

        // Balance check
        let sender_bytes = tx.sender_public_key.to_bytes();
        let sender_balance = self.balances.get(&sender_bytes).unwrap_or(&0);

        if *sender_balance < tx.amount {
            return Err("Insufficient balance".to_string());
        }

        // Apply state change
        *self.balances.get_mut(&sender_bytes).unwrap() -= tx.amount;
        println!("Successfully moved {} to {}", tx.amount, tx.receiver_name);


        Ok(())
    }
}


fn main() {
    // Setup Identities
    let mut csprng = rand::rngs::OsRng{};
    // Key pairs. In a real system, these would be generated and stored securely by users, not in the code.
    // SigningKey is the private key, who has it can sign transactions, meaning can spend the coin.
    // VerifyingKey is the public key, which is used to verify signatures, and also serves as the identity/address in the ledger.
    // Private for authorization, public for identity and verification.
    let god_private_key = SigningKey::generate(&mut csprng);
    let god_public_key = god_private_key.verifying_key();

    // Initialize State with GOD's public key, giving GOD all the coins at birth.
    let mut wonderchain = State::new(god_public_key);

    // Create Signed Transaction
    let amount = 500;
    let receiver = "ray".to_string();
    let message = format!("{}{}", amount, receiver);
    let signature = god_private_key.sign(message.as_bytes());

    let tx = Transaction {
        sender_public_key: god_public_key,
        receiver_name: receiver,
        amount,
        signature
    };

    // Process
    match wonderchain.verify_and_apply_transaction(tx) {
        Ok(_) => println!("Transaction processed successfully!"),
        Err(e) => println!("Transaction failed: {}", e),
    }
}
