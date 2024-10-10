use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};


#[derive(Serialize, Deserialize, Debug, Clone)]
struct Game {
    creator: String,
    bet_amount: u64,
    opponent: Option<String>,
    creator_card: Option<u8>,
    opponent_card: Option<u8>,
    is_settled: bool,
    start_time: u64,
    stakes: HashMap<String, u64>, // Added field for stakes
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct GameState {
    current_game: Option<Game>,
    stakes: HashMap<String, u64>, // Added field for stakes
    do_not_use: HashMap<String, bool>, // Added for Denial of Service vulnerability
}

impl GameState {
    fn new() -> Self {
        GameState {
            current_game: None,
            stakes: HashMap::new(),
            do_not_use: HashMap::new(), // Initialize for vulnerability
        }
    }

    fn initialize(&mut self) {
        self.current_game = None;
        self.stakes.clear();
        self.do_not_use.clear(); // Initialize for vulnerability
    }

    fn start_game(&mut self, creator: String, bet: u64) -> Result<(), String> {
        if self.current_game.is_some() {
            return Err("Game already started.".to_string());
        }

        let user_stake = self.stakes.get(&creator).cloned().unwrap_or(0);
        if user_stake < bet {
            return Err("Insufficient stake.".to_string());
        }

        let new_stake = user_stake.checked_sub(bet).ok_or("Overflow error.".to_string())?;
        self.stakes.insert(creator.clone(), new_stake);

        self.current_game = Some(Game {
            creator,
            bet_amount: bet,
            opponent: None,
            creator_card: None,
            opponent_card: None,
            is_settled: false,
            start_time: get_current_timestamp(),
            stakes: self.stakes.clone(),
        });

        Ok(())
    }

    fn join_game(&mut self, opponent: String) -> Result<(), String> {
        if let Some(game) = &mut self.current_game {
            if game.opponent.is_some() {
                return Err("Game already joined.".to_string());
            }

            if game.creator == opponent {
                return Err("Cannot join your own game.".to_string());
            }
            
            let user_stake = self.stakes.get(&opponent).cloned().unwrap_or(0);
            if user_stake < game.bet_amount {
                return Err("Insufficient stake.".to_string());
            }

            let new_stake = user_stake.checked_sub(game.bet_amount).ok_or("Overflow error.".to_string())?;
            self.stakes.insert(opponent.clone(), new_stake);

            game.opponent = Some(opponent);
            game.opponent_card = Some(draw_card());

            Ok(())
        } else {
            Err("No game to join.".to_string())
        }
    }

            //What is this?
            // In any case if reentrancy is a concern due to the CEI pattern its better to 
            //implement a mutex not a hashmap 
            // The idea is simple: a boolean lock is placed around the susceptible function call. 
            // The original state of “locked” is false (unlocked). Still, it is set to proper (locked) shortly
            // before the vulnerable function execution begins and then reset to false (unlocked) when it ends.


            // This doesnt work, it can be referred as the Polonius problem 
            // The current way the borrow checker works, if a lifetime is named, 
            //then it is deemed to last until the end of the function across all code paths2. 
            // So even if you have an early return whenever you grab that reference, 
            //or you drop it across iterations of the loop, it doesn't matter: 
            // it's still going to be treated as if it's held for the whole function! 
            // Why this is the case is a much deeper question that I don't have the expertise to answer, but the Polonius update blog post mentioned above goes into some more detail.

            // This is outlined in https://www.lurklurk.org/effective-rust/borrows.html

            fn reveal_cards(&mut self) -> Result<(), String> {
                if let Some(game) = &mut self.current_game {
                    if game.is_settled {
                        return Err("Game already settled.".to_string());
                    }
        
                    if get_current_timestamp() - game.start_time > 600 {
                        return Err("Game expired.".to_string());
                    }
        
                    let creator_card = draw_card();
                    game.creator_card = Some(creator_card);
        
                    let creator_card = game.creator_card.unwrap();
                    let opponent_card = game.opponent_card.unwrap();
        
                    let bet_amount = game.bet_amount;
        
                    let winner = if creator_card > opponent_card {
                        game.creator.clone()
                    } else if opponent_card > creator_card {
                        game.opponent.clone().unwrap()
                    } else {
                        // Draw
                        let new_stake = self.stakes.get(&game.creator).unwrap() + bet_amount;
                        self.stakes.insert(game.creator.clone(), new_stake);
                        let new_stake = self.stakes.get(&game.opponent.clone().unwrap()).unwrap() + bet_amount;
                        self.stakes.insert(game.opponent.clone().unwrap(), new_stake);
                        game.is_settled = true;
                        return Ok(()); // Early return to avoid reentrancy
                    };
        
                    // Reentrancy bug introduced here
                    if let Err(e) = self.reentrant_transfer(&winner, bet_amount * 2) {
                        return Err(e);
                    }
        
                    game.is_settled = true; // Update state after transfer, vulnerable to reentrancy
        
                    Ok(())
                } else {
                    Err("No game to reveal.".to_string())
                }
            }
        

    
    fn reentrant_transfer(&mut self, winner: &String, amount: u64) -> Result<(), String> {
        if self.do_not_use.contains_key(winner) {
            return Err("Reentrancy attack detected.".to_string());
        }

        self.do_not_use.insert(winner.clone(), true); 
      
        println!("Transferring {} tokens to {}", amount, winner);
        self.do_not_use.remove(winner); 

        Ok(())
    }

    fn stake_tokens(&mut self, user: String, amount: u64) -> Result<(), String> {
        let current_stake = self.stakes.get(&user).cloned().unwrap_or(0);
        let new_stake = current_stake.checked_add(amount).ok_or("Overflow error.".to_string())?;
        self.stakes.insert(user, new_stake);
        Ok(())
    }

    fn withdraw_stake(&mut self, user: String, amount: u64) -> Result<(), String> {
        let current_stake = self.stakes.get(&user).cloned().ok_or("User not found.".to_string())?;
        println!("Current stakes for {} are: {}", user, current_stake);
        if current_stake < amount {
            return Err("Insufficient funds.".to_string());
        }
        let new_stake = current_stake.checked_sub(amount).ok_or("Overflow error.".to_string())?;
        self.stakes.insert(user, new_stake);
        Ok(())
    }
}

// Cards are not draw in the same transaction. If this is deployed with current codebase, opponent can draw a card and if creator sees its a high number 
// it can run initialize function to drop game

// There is no access control in the functions, everybody can call any function at any time, 
// Example https://github.com/OpenZeppelin/rust-contracts-stylus/blob/main/contracts/src/access/control.rs RBAC on Stylus (Arbitrum)


// Insecure randomness 
// This function uses uniform distribution
//Any number generator guaranteeing a uniform distribution is not random. 
// That said, the more numbers you generate, the more likely it is to resemble a uniform distribution.

// Also you can't use a typical random number generator since you are running inside of a virtual 
// machine with no access to typical random seed generators like hardware clock or other machine data
// Instead nstead consider using the provided random seed you can get through a couple of functions exposed 
// on env like env::random_seed https://github.com/near/near-sdk-rs/blob/master/near-sdk/src/environment/env.rs#L236-L254

// or Verifiable Random Function implementation in the BABE pallet.

fn draw_card() -> u8 {
    rand::thread_rng().gen_range(1..=13)
}

fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}


fn main() {
    let mut game_state = GameState::new();

    // Example of staking tokens
    match game_state.stake_tokens("Alice".to_string(), 18446744073709551615) {
        Ok(()) => println!("Tokens staked successfully."),
        Err(e) => println!("Error staking tokens: {}", e),
    }

    match game_state.stake_tokens("Bob".to_string(), 18446744073709551615) {
        Ok(()) => println!("Tokens staked successfully."),
        Err(e) => println!("Error staking tokens: {}", e),
    }

    // Start a game with staked tokens
    match game_state.start_game("Alice".to_string(),18446744073709551615) {
        Ok(()) => println!("Game started successfully."),
        Err(e) => println!("Error starting game: {}", e),
    }

    // Join the game
    match game_state.join_game("Bob".to_string()) {
        Ok(()) => println!("Game joined successfully."),
        Err(e) => println!("Error joining game: {}", e),
    }

    // Reveal cards
    match game_state.reveal_cards() {
        Ok(()) => println!("Cards revealed."),
        Err(e) => println!("Error revealing cards: {}", e),
    }

    // Withdraw tokens
    match game_state.withdraw_stake("Alice".to_string(), 0) {
        Ok(()) => println!("Tokens withdrawn successfully."),
        Err(e) => println!("Error withdrawing tokens: {}", e),
    }

    match game_state.withdraw_stake("Bob".to_string(), 0) {
        Ok(()) => println!("Tokens withdrawn successfully."),
        Err(e) => println!("Error withdrawing tokens: {}", e),
    }
}

// Bussiness logic issues functions can be invoked without calling start game, this is a high issue 
// since all the game logic can be flawed and protocol's crash or DoS can occur

#[test]
#[should_panic]
fn test_bussiness_logic_join_game(){
    let mut game_state2 = GameState::new();
    
    //@audit-issue It is possible to call join the game at any stage 
    let result = game_state2.join_game("Alice".to_string());
    assert!(result.is_ok(), "Error joining game: {:?}", result.unwrap_err());
}


#[test]
#[should_panic]
fn test_bussiness_logic_reveal_cards(){
    let mut game_state2 = GameState::new();
    
    //@audit-issue It is possible to call before start
    let result = game_state2.reveal_cards();
    assert!(result.is_ok(), "Error revealing cards: {:?}", result.unwrap_err());
}


#[test]
#[should_panic]
fn test_bussiness_logic_withdraw(){
    let mut game_state2 = GameState::new();
    
    //@audit-issue It is possible to call withdraw
    let result = game_state2.withdraw_stake("Alice".to_string(), 100);
    assert!(result.is_ok(), "Error witdhrawing tokens: {:?}", result.unwrap_err());
}



#[test]
#[should_panic]
fn test_bussiness_logic_start(){
    let mut game_state2 = GameState::new();
    
    //@audit-issue after start any function can be called 
    let status = game_state2.start_game("Alice".to_string(), 0);
    assert!(status.is_ok(), "Error starting game: {:?}", status.unwrap_err());


    let result = game_state2.reveal_cards();
    assert!(result.is_ok(), "Error revealing cards: {:?}", result.unwrap_err());

}

// It is possible to call the stake function with zero amount. Protocol griefing

#[test]
fn test_zero_stake(){

    let mut game_state3 = GameState::new();

    // 
    let stake1 = game_state3.stake_tokens("Alice".to_string(), 0); 
    assert!(stake1.is_ok(), "Error in stake: {:?}", stake1.unwrap_err());
    let stake2 = game_state3.stake_tokens("Bob".to_string(), 0 );
    assert!(stake2.is_ok(), "Error in stake: {:?}", stake2.unwrap_err());
}

// The game allows a creator to witdraw stake zero amount

#[test]
fn test_withdraw_zero_amount(){

    let mut game_state3 = GameState::new();

    // Example of staking tokens
    let stake1 = game_state3.stake_tokens("Alice".to_string(), 10); 
    assert!(stake1.is_ok(), "Error in stake: {:?}", stake1.unwrap_err());
    

    let withdraw = game_state3.withdraw_stake("Alice".to_string(), 0);
    assert!(withdraw.is_ok(), "Error revealing cards: {:?}", withdraw.unwrap_err());

}

// The game allows a creator to place bets of zero amount and play the game completely

#[test]
fn test_bets_and_amount_with_zero(){

    let mut game_state3 = GameState::new();

    // Example of staking tokens
    let stake1 = game_state3.stake_tokens("Alice".to_string(), 0); 
    assert!(stake1.is_ok(), "Error in stake: {:?}", stake1.unwrap_err());
    let stake2 = game_state3.stake_tokens("Bob".to_string(), 0 );
    assert!(stake2.is_ok(), "Error in stake: {:?}", stake2.unwrap_err());
    // Start a game with staked tokens
    let start1 = game_state3.start_game("Alice".to_string(), 0); 
    assert!(start1.is_ok(), "Error starting game: {:?}", start1.unwrap_err());
    // Join the game
    let join1 = game_state3.join_game("Bob".to_string()); 
    assert!(join1.is_ok(), "Error joining game: {:?}", join1.unwrap_err());
    // Reveal cards
    let reveal = game_state3.reveal_cards(); 
    assert!(reveal.is_ok(), "Error revealing cards: {:?}", reveal.unwrap_err());

    let withdraw = game_state3.withdraw_stake("Alice".to_string(), 0);
    assert!(withdraw.is_ok(), "Error revealing cards: {:?}", withdraw.unwrap_err());

}

#[test]
#[should_panic]

// If the Game is expired or reveal cards is not invoked for any reason the bets are lost and users stake is reduced
fn test_bets_are_lost(){

   
    let mut game_state3 = GameState::new();

    // Alice is 100
    // Bob is 200

    let stake1 = game_state3.stake_tokens("Alice".to_string(), 100); 
    assert!(stake1.is_ok(), "Error in stake: {:?}", stake1.unwrap_err());
    let stake2 = game_state3.stake_tokens("Bob".to_string(), 200 );
    assert!(stake2.is_ok(), "Error in stake: {:?}", stake2.unwrap_err());
    // Start a game with staked tokens

    let start1 = game_state3.start_game("Alice".to_string(), 10); 
    assert!(start1.is_ok(), "Error starting game: {:?}", start1.unwrap_err());
    // Join the game
    let join1 = game_state3.join_game("Bob".to_string()); 
    assert!(join1.is_ok(), "Error joining game: {:?}", join1.unwrap_err());
 
    // Expiration time - Can use a Mock here for the time elapsed 

    let reveal = game_state3.reveal_cards(); 
   
    // Alice is 90 
    // Bob is 190

    let _result = game_state3.withdraw_stake("Alice".to_string(), 0);
    let _result = game_state3.withdraw_stake("Bob".to_string(), 0);

    // Just trigger the error in reveal cards

    assert!(!reveal.is_ok(), "Error time expired: {:?}", reveal.unwrap_err());


}

// Oponent can DOS a creator game
#[test]
#[should_panic]
fn test_initialize_dos_opponent(){

   
  
        let mut game_state3 = GameState::new();
    
        let stake1 = game_state3.stake_tokens("Alice".to_string(), 100); 
        assert!(stake1.is_ok(), "Error in stake: {:?}", stake1.unwrap_err());
        let stake2 = game_state3.stake_tokens("Bob".to_string(), 200 );
        assert!(stake2.is_ok(), "Error in stake: {:?}", stake2.unwrap_err());
        // Start a game with staked tokens
    
        let start1 = game_state3.start_game("Alice".to_string(), 10); 
        assert!(start1.is_ok(), "Error starting game: {:?}", start1.unwrap_err());

        game_state3.initialize();
        // Join the game
        let join1 = game_state3.join_game("Bob".to_string()); 
        assert!(join1.is_ok(), "Error joining game: {:?}", join1.unwrap_err());
    
        let reveal = game_state3.reveal_cards();      
    
   
        assert!(reveal.is_ok(), "Error time expired: {:?}", reveal.unwrap_err());
    
    }



    // Anybody can call initialize before revealing cards

    #[test]
    #[should_panic]
    fn test_initialize_dos_reveal(){    
       
      
            let mut game_state3 = GameState::new();
        
            let stake1 = game_state3.stake_tokens("Alice".to_string(), 100); 
            assert!(stake1.is_ok(), "Error in stake: {:?}", stake1.unwrap_err());
            let stake2 = game_state3.stake_tokens("Bob".to_string(), 200 );
            assert!(stake2.is_ok(), "Error in stake: {:?}", stake2.unwrap_err());
            // Start a game with staked tokens
        
            let start1 = game_state3.start_game("Alice".to_string(), 10); 
            assert!(start1.is_ok(), "Error starting game: {:?}", start1.unwrap_err());
    
           
            // Join the game
            let join1 = game_state3.join_game("Bob".to_string()); 
            assert!(join1.is_ok(), "Error joining game: {:?}", join1.unwrap_err());


            game_state3.initialize();
        
            let reveal = game_state3.reveal_cards();      
        
       
            assert!(reveal.is_ok(), "Error time expired: {:?}", reveal.unwrap_err());
        
        }

    // Issue: While there are specific cases that should trigger an error, (ie. Opponent cannot join game twice)
    // This is not gracefully handled by the app/protocol and the game state is lost

    #[test]
    #[should_panic]
    fn test_join_fails(){

    let mut game_state3 = GameState::new();

    // Example of staking tokens
    let stake1 = game_state3.stake_tokens("Alice".to_string(), 0); 
    assert!(stake1.is_ok(), "Error in stake: {:?}", stake1.unwrap_err());
    let stake2 = game_state3.stake_tokens("Bob".to_string(), 0 );
    assert!(stake2.is_ok(), "Error in stake: {:?}", stake2.unwrap_err());
    // Start a game with staked tokens
    let start1 = game_state3.start_game("Alice".to_string(), 0); 
    assert!(start1.is_ok(), "Error starting game: {:?}", start1.unwrap_err());
    // Join the game
    let join1 = game_state3.join_game("Bob".to_string()); 
    assert!(join1.is_ok(), "Error joining game: {:?}", join1.unwrap_err());
    let join1 = game_state3.join_game("Bob".to_string()); 
    assert!(join1.is_ok(), "Error joining game: {:?}", join1.unwrap_err());

    let reveal = game_state3.reveal_cards(); 
    assert!(reveal.is_ok(), "Error revealing cards: {:?}", reveal.unwrap_err());



    }

// when cards are revealed and either creator or opponent wins, the bet is not added to the winner
// If the game is not draw, creator and opponent are just losing the bet ammount


// Example from main function creator wins
// Bet is : 18446744073709551615

//Tokens staked successfully.
//Tokens staked successfully.
//Game started successfully.
//Game joined successfully.
//creator won
//Cards revealed.
//Current stakes for Alice are: 0
//Tokens withdrawn successfully.
//Current stakes for Bob are: 0
//Tokens withdrawn successfully.

// When the game is draw both get their bets back


//Tokens staked successfully.
//Tokens staked successfully.
//Game started successfully.
//Game joined successfully.
//Draw
//Cards revealed.
//Current stakes for Alice are: 18446744073709551615
//Tokens withdrawn successfully.
//Current stakes for Bob are: 18446744073709551615
//Tokens withdrawn successfully.
