use std::collections::HashMap;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Sha256, Digest};
use std::fmt::Write;

const UNIQUE_SUPPLY: u64 = 1000000;

struct Blockchain {
    chain: Vec<Block>,
    state: State,
}

impl Blockchain {
    fn new(god_name: &str, god_key: VerifyingKey) -> Self {
        let state = State::new(god_name, god_key);
        let mut blockchain = Blockchain {
            chain: Vec::new(),
            state,
        };

        // Create Genesis Block
        let genesis_prev_hash = "0".repeat(64); // 64 hex chars for SHA-256
        let genesis_block = Block::new(
            genesis_prev_hash,
            0,
            Vec::new(), // No transactions in genesis block
        );

        blockchain.chain.push(genesis_block);
        blockchain
    }

    fn add_transaction(&mut self, tx: Transaction) -> Result<(), String> {
        // Try to apply it to our state
        // Reach into self.state
        self.state.process_transaction(tx.clone())?;

        // If successful, we can add it to the latest block
        let prev_hash = self.chain.last().unwrap().header.hash.clone();
        let new_height = self.chain.len() as u64;

        let new_block = Block::new(
            prev_hash,
            new_height,
            vec![tx],
        );

        // Append to chain
        self.chain.push(new_block);

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct Block {
    header: BlockHeader,
    transactions: Vec<Transaction>,
}

#[derive(Debug, Clone)]
struct BlockHeader {
    previous_hash: String,
    block_height: u64,
    nonce: u32,
    hash: String,
}

impl Block {
    // Utility to calculate hash of the block data
    fn calculate_hash(
        previous_hash: &str,
        block_height: u64,
        nonce: u32,
        transactions: &[Transaction],
    ) -> String {
        let mut hasher = Sha256::new();

        // Hash header info + representation of transactions
        let input = format!("{}{}{}{:?}", previous_hash, block_height, nonce, transactions);
        hasher.update(input);

        let result = hasher.finalize();
        let mut s = String::new();
        for byte in result {
            write!(&mut s, "{:02x}", byte).expect("Unable to write");
        }
        s
    }

    fn new(previous_hash: String, block_height: u64, transactions: Vec<Transaction>) -> Block {
        let hash = Self::calculate_hash(&previous_hash, block_height, 0, &transactions);
        Block {
            header: BlockHeader {
                previous_hash,
                block_height,
                nonce: 0,
                hash,
            },
            transactions,
        }
    }
}

#[derive(Debug, Clone)]
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

    // Setup Chain
    let mut wonderchain = Blockchain::new("GOD", god_public_key);

    // Setup Ray
    let ray_private_key = SigningKey::generate(&mut csprng);
    let ray_public_key = ray_private_key.verifying_key();

    // Create Transaction
    let tx = Transaction {
        sender_name: "GOD".to_string(),
        receiver_name: "ray".to_string(),
        receiver_public_key: Some(ray_public_key),
        amount: 500,
        signature: god_private_key.sign(format!("{}{}{}", "GOD", "ray", 500).as_bytes()),
    };

    // Execute
    match wonderchain.add_transaction(tx) {
        Ok(_) => {
            println!("Block #1 Added!");
            println!("New Block Hash: {}", wonderchain.chain.last().unwrap().header.hash);
            println!("GOD Balance: {}", wonderchain.state.balances.get("GOD").unwrap());
            println!("Ray Balance: {}", wonderchain.state.balances.get("ray").unwrap());
        },
        Err(e) => println!("Transaction failed: {}", e),
    }
}
