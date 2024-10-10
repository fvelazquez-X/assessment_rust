use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ERC20Token {
    owner: String,
    balances: HashMap<String, u64>,
    mint_price: f64, // Price per token in ETH
}

impl ERC20Token {
    fn new(owner: String) -> Self {
        ERC20Token {
            owner,
            balances: HashMap::new(),
            mint_price: 0.001, // Initial price per token in ETH
        }
    }

    // Does not follow CEI pattern 
    fn mint(&mut self, user: String, amount: u64, eth_paid: f64) -> Result<(), String> {
        if eth_paid < amount as f64 * self.mint_price {
            return Err("Insufficient ETH paid.".to_string());
        }

        let current_balance = self.balances.entry(user.clone()).or_insert(0);
        *current_balance += amount;

        Ok(())
    }

    // Does not follow CEI pattern
    // In this case, if this would be a transfer() call, another contract can recursively call the function it could repeatedly drain funds.

    fn transfer(&mut self, from: String, to: String, amount: u64) -> Result<(), String> {
        let from_balance = self.balances.get_mut(&from).ok_or("Sender not found.".to_string())?;
        if *from_balance < amount {
            return Err("Insufficient balance.".to_string());
        }

        let to_balance = self.balances.entry(to.clone()).or_insert(0);
        *from_balance -= amount;
        *to_balance += amount;

        Ok(())
    }

    fn adjust_price(&mut self, new_price: f64) {
        // Vulnerability: No access control
        self.mint_price = new_price;
    }

    fn get_balance(&self, user: &String) -> u64 {
        self.balances.get(user).cloned().unwrap_or(0)
    }
}

fn main() {
    let owner = "OwnerAddress".to_string();
    let mut token = ERC20Token::new(owner.clone());

    // Mint tokens
    match token.mint("User1".to_string(), 100, 0.1) {
        Ok(()) => println!("Minted tokens successfully."),
        Err(e) => println!("Error minting tokens: {}", e),
    }

    // Adjust price (vulnerable to any user)
    token.adjust_price(0.002);
    println!("New mint price set to: {}", token.mint_price);

    // Transfer tokens
    match token.transfer("User1".to_string(), "User2".to_string(), 50) {
        Ok(()) => println!("Tokens transferred successfully."),
        Err(e) => println!("Error transferring tokens: {}", e),
    }

    // Get balances
    println!("User1 balance: {}", token.get_balance(&"User1".to_string()));
    println!("User2 balance: {}", token.get_balance(&"User2".to_string()));
}