#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod polymarket{
    use ink:: {env::debug_println, prelude::vec::Vec, storage::Mapping};
    pub type Result<T> = core::result::Result<T, Error>;

   #[ink(storage)]
    pub struct PolymarketStore{
        user_response_mapping:Mapping<Vec<u8>, UserResponseStorage>, 
        question_info:Mapping<Hash,QuestionStorage>
    }

    #[derive(scale::Decode, scale::Encode, Debug)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]

    pub struct UserResponseStorage{
        response :bool,
        betting_amount:Balance,
    }

    #[derive(scale::Decode, scale::Encode, Debug)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]

    pub struct QuestionStorage{
        question_id:Hash,
        question_creation_time:Timestamp,
        question_expiration_time:Timestamp,
        question_locking_period:Timestamp,
        status:Status
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo,ink::storage::traits::StorageLayout))]
    pub enum Status{
         QuestionRunning,
         LockingPeriodStart
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error{
        InvalidCredentials,
        QuestionAlreadyExists,
        QuestionDoesNotExists
    }

    #[ink(event)]
    pub struct EmitResponse{
        #[ink(topic)]
        user_id:AccountId,
        #[ink(topic)]
        question_id:Hash,
        user_response:bool,
        betting_amount:Balance
    }

    impl PolymarketStore{
    #[ink(constructor)]
    pub fn new_()->Self{
    Self{
        user_response_mapping : Mapping::default(),
        question_info : Mapping::default()
    }
    }

// Function to Register Betting Question
    #[ink(message)]
    pub fn register_question(&mut self,question_id:Hash,question_creation_time:Timestamp,question_expiration_time:Timestamp,question_locking_period:Timestamp)->Result<QuestionStorage>{
         if !self.question_info.contains(question_id){
         let question=QuestionStorage{
            question_id,
            question_creation_time,
            question_expiration_time,
            question_locking_period,
            status:Status::QuestionRunning
        };
    self.question_info.insert(question_id,&question);
    Ok(question)
         }
         else{
            Err(Error::QuestionAlreadyExists)
         }
    }

// Function to Bet on a Question until question is running
    #[ink(message,payable)]
    pub fn bet_question(&mut self,user_id:AccountId,question_id:Hash,user_response:bool,betting_amount:Balance)->Result<()>{
        let question_expiration_time=self.question_info.get(question_id).unwrap().question_expiration_time;
        let new_id=self.concatenate_account_id_and_hash(&user_id, &question_id);
        let current_time=self.env().block_timestamp()/1000;
        debug_println!("block time:{}",current_time);
        debug_println!("expiration time:{}",question_expiration_time);

    if (current_time<=question_expiration_time)&&(self.user_response_mapping.contains(&new_id))&&(self.env().transferred_value()==betting_amount){
        let current_betting_amount=self.user_response_mapping.get(&new_id).unwrap().betting_amount;
        let new_betting_amount=current_betting_amount.checked_add(betting_amount);
        let response_store=UserResponseStorage{
            response:user_response,
            betting_amount:new_betting_amount.unwrap()
        };
        self.user_response_mapping.insert(&new_id,&response_store);

        self.env().emit_event(EmitResponse{
            user_id,
            question_id,
            user_response,
            betting_amount:new_betting_amount.unwrap()
        });
        Ok(())
    }
    else if (current_time<=question_expiration_time)&&(!self.user_response_mapping.contains(&new_id))&&(self.env().transferred_value()==betting_amount) {
        let response_store=UserResponseStorage{
            response:user_response,
            betting_amount
        };
        self.user_response_mapping.insert(&new_id,&response_store);

        self.env().emit_event(EmitResponse{
            user_id,
            question_id,
            user_response,
            betting_amount
        });
        Ok(())
    }
        else{
            Err(Error::InvalidCredentials)
        }
    }

// Function to Withdraw Bet of user on a Question until question is running
    #[ink(message)]
    pub fn withdraw_bet(&mut self,user_id:AccountId,question_id:Hash)->Result<()>{
        let question_expiration_time=self.question_info.get(question_id).unwrap().question_expiration_time;
        let current_time=self.env().block_timestamp()/1000;
        debug_println!("time:{}",current_time);
        debug_println!("expiration time:{}",question_expiration_time);
        let new_id=self.concatenate_account_id_and_hash(&user_id, &question_id);
        let current_betting_amount=self.user_response_mapping.get(&new_id).unwrap().betting_amount;
        let user_response=self.user_response_mapping.get(&new_id).unwrap().response;

        if current_time<=question_expiration_time{
            self.env().transfer(user_id,current_betting_amount).unwrap();
            let response_store=UserResponseStorage{
                response:user_response,
                betting_amount:0
            };
            self.user_response_mapping.insert(new_id,&response_store);
        Ok(())
        }

        else{
        Err(Error::InvalidCredentials)
        }
    }

// Function to read User_response on a Question
    #[ink(message)]
    pub fn read_user_response(&mut self,user_id:AccountId,question_id:Hash)->Result<UserResponseStorage>{
        let new_id=self.concatenate_account_id_and_hash(&user_id, &question_id);
        if let Some(response)=self.user_response_mapping.get(new_id){
            Ok(response)
        }
        else{
            Err(Error::InvalidCredentials)
        }
    }

// Function to read status of a Question as if Question is running or Question is in Locking Period
    #[ink(message)]
    pub fn read_question_status(&mut self,question_id:Hash)->Result<QuestionStorage>{
        let current_time=self.env().block_timestamp()/1000;
        let val =self.question_info.get(question_id);

        if val.is_none(){
            return Err(Error::QuestionDoesNotExists) 
        }

        let question_expiration_time = val.unwrap().question_expiration_time;

        if current_time > question_expiration_time {
        let question_store1=self.question_info.get(question_id).unwrap();
        let question_creation_time=self.question_info.get(question_id).unwrap().question_creation_time;
        let question_locking_period=self.question_info.get(question_id).unwrap().question_locking_period;
        let question_store=QuestionStorage{
            question_id,
            question_creation_time,
            question_expiration_time,
            question_locking_period,
            status:Status::LockingPeriodStart
         };

         self.question_info.insert(question_id,&question_store);
         Ok(question_store1)

        }

        else {
            let question_store1=self.question_info.get(question_id).unwrap();
            Ok(question_store1)
         }

    }

// Helper Function
    pub fn concatenate_account_id_and_hash(&self,account_id: &AccountId, hash: &Hash) -> Vec<u8> {
        let mut concatenated: Vec<u8> = Vec::new();

        concatenated.extend_from_slice(account_id.as_ref());
        concatenated.extend_from_slice(hash.as_ref());
        debug_println!("output: {:?}",concatenated);

        concatenated
    }
        }
        }

