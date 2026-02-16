use std::collections::HashMap;


const UNIQUE_SUPPLY: u64 = 1000000;

// Global Ledger
struct State {
    balances: HashMap<String, u64>
}

impl State {
    fn new() -> Self {
        let mut balances = HashMap::new();
        // Rule 1 - All coins exists at birth in GOD
        balances.insert("GOD".to_string(), UNIQUE_SUPPLY);


        State { balances }
    }

    // Simple test transfer
    fn apply_transaction(&mut self, from: &str, to: &str, amount: u64) -> Result<(), String> {
        let sender_balance = self.balances.get(from).unwrap_or(&0);

        if *sender_balance < amount {
            return Err("Insecure funds: God is not that generous today".to_string());
        }

        // Deduct and Add
        // Ensures total supply invariant
        *self.balances.entry(from.to_string()).or_insert(0) -= amount;
        *self.balances.entry(to.to_string()).or_insert(0) += amount;

        Ok(())
    }
}


fn main() {
    let mut wonderchain = State::new();
    println!("Initial GOD balance: {}", wonderchain.balances["GOD"]);

    // Try give Alice 100 coins
    match wonderchain.apply_transaction("GOD", "Alice", 100) {
        Ok(_) => println!("Transaction successful! Alice has received 100 coins."),
        Err(e) => println!("Transaction failed: {}", e),
    }

    println!("Final GOD balance: {}", wonderchain.balances["GOD"]);
}
