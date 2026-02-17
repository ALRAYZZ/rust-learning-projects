use std::collections::HashMap;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

const UNIQUE_SUPPLY: u64 = 1000000;

struct Transaction {
    sender_name: String,
    receiver_name: String,
    receiver_public_key: Option<VerifyingKey>,
    amount: u64,
    // Only private key-holder can produce signature, anyone with the public key can verify it,
    // proving authenticity and authorization of the transaction.
    signature: Signature,
}


// Global Ledger
struct State {
    // Names -> Balance
    balances: HashMap<String, u64>, // Every key must be a 32-byte array (Ed25519 public key size)
    // Name -> Public Key (Phonebook)
    names: HashMap<String, VerifyingKey>,
}

impl State {
    fn new(god_name: &str, god_key: VerifyingKey) -> Self {
        let mut balances = HashMap::new();
        let mut names = HashMap::new();
        // Rule 1 - All coins exists at birth in GOD
        balances.insert(god_name.to_string(), UNIQUE_SUPPLY);
        names.insert(god_name.to_string(), god_key);

        State { balances, names }
    }

    fn process_transaction(&mut self, tx: Transaction) -> Result<(), String> {
        // Look up sender key
        let sender_key = self.names.get(&tx.sender_name)
            .ok_or("Sender does not exist!");

        // Verify signature
        let message = format!("{}{}{}", tx.sender_name, tx.receiver_name, tx.amount);
        sender_key?.verify(message.as_bytes(), &tx.signature)
            .map_err(|_| "Invalid signature!")?;

        // Check Balance
        let sender_balance = self.balances.get(&tx.sender_name).cloned().unwrap_or(0);
        if sender_balance < tx.amount {
            return Err("Insufficient balance!".to_string());
        }

        // Handle receiver registration if needed
        if !self.names.contains_key(&tx.receiver_name) {
            // If account is new, we require public key to lock it
            let new_key = tx.receiver_public_key
                .ok_or("New accounts must provide a Public Key to register!")?;

            self.names.insert(tx.receiver_name.clone(), new_key);
            self.balances.insert(tx.sender_name.clone(), sender_balance);
        }

        // Update state
        *self.balances.get_mut(&tx.sender_name).unwrap() -= tx.amount;
        *self.balances.entry(tx.receiver_name.clone()).or_insert(0) += tx.amount;

        Ok(())
    }
}


fn main() {
    // Setup Identities
    let mut csprng = rand::rngs::OsRng;
    // Key pairs. In a real system, these would be generated and stored securely by users, not in the code.
    // SigningKey is the private key, who has it can sign transactions, meaning can spend the coin.
    // VerifyingKey is the public key, which is used to verify signatures, and also serves as the identity/address in the ledger.
    // Private for authorization, public for identity and verification.

    // Setup GOD
    let god_private_key = SigningKey::generate(&mut csprng);
    let god_public_key = god_private_key.verifying_key();
    let mut wonderchain = State::new("GOD", god_public_key);

    // Setup Ray
    let ray_private_key = SigningKey::generate(&mut csprng);
    let ray_public_key = ray_private_key.verifying_key();

    // Create Signed Transaction
    let amount = 500;
    let sender = "GOD".to_string();
    let receiver = "ray".to_string();

    // Message must include sender name to prevent replay attacks
    let message = format!("{}{}{}", sender, receiver, amount);
    let signature = god_private_key.sign(message.as_bytes());

    let tx = Transaction {
        sender_name: sender,
        receiver_name: receiver,
        receiver_public_key: Some(ray_public_key),
        amount,
        signature
    };

    // Execute
    match wonderchain.process_transaction(tx) {
        Ok(_) => {
            println!("Transaction processed successfully!");
            println!("God: {} | Alice: {}",
            wonderchain.balances["GOD"],
            wonderchain.balances["ray"]
            );
        },
        Err(e) => println!("Transaction failed: {}", e),
    }
}
