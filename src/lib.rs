use near_sdk::store::LookupMap;
use near_sdk::json_types::U128;
use near_sdk::{
    near_bindgen, AccountId, NearToken, PanicOnDefault, Promise, env,
};
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde_json;

const MINT_PRICE: u128 = 500_000_000_000_000_000_000_000;

#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct TokenMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub media: Option<String>,
    pub media_hash: Option<String>,
}

#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct Token {
    pub token_id: String,
    pub owner_id: AccountId,
    pub metadata: TokenMetadata,
    pub revealed: bool,
    pub sold: bool,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct SpellbookNFT {
    pub owner_id: AccountId,
    pub tokens: LookupMap<String, Token>,
    pub token_list: Vec<String>,
    pub hidden_uri: String,
    pub treasury: AccountId,
}

#[near_bindgen]
impl SpellbookNFT {
    #[init]
    pub fn new(hidden_uri: String) -> Self {
        Self {
            owner_id: env::predecessor_account_id(),
            tokens: LookupMap::new(b"t"),
            token_list: Vec::new(),
            hidden_uri,
            treasury: env::predecessor_account_id(),
        }
    }

    #[init]
    pub fn new_default_meta(hidden_uri: String) -> Self {
        Self::new(hidden_uri)
    }

    pub fn nft_metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "spec": "nft-1.0.0",
            "name": "Monad Spellbook",
            "symbol": "SPELL",
            "icon": null,
            "base_uri": null,
            "reference": null,
            "reference_hash": null
        })
    }

    fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "Owner only"
        );
    }

    pub fn mint_token(&mut self, token_id: String, title: String, media: String, unhide_media: String) {
        self.assert_owner();
        assert!(!self.tokens.contains_key(&token_id), "Token already exists");
        let token = Token {
            token_id: token_id.clone(),
            owner_id: self.owner_id.clone(),
            metadata: TokenMetadata {
                title: Some(title),
                description: Some(format!("Monad Spellbook NFT - {}", token_id)),
                media: Some(unhide_media),
                media_hash: None,
            },
            revealed: false,
            sold: false,
        };
        self.tokens.insert(token_id.clone(), token);
        self.token_list.push(token_id);
    }

    #[payable]
    pub fn buy_token(&mut self, token_id: String) -> Promise {
        let deposit = env::attached_deposit();
        assert!(deposit.as_yoctonear() >= MINT_PRICE, "Attach at least 0.5 NEAR");
        let mut token = self.tokens.get(&token_id).expect("Token not found").clone();
        assert!(!token.sold, "Token already sold");
        token.sold = true;
        token.revealed = true;
        token.owner_id = env::predecessor_account_id();
        self.tokens.insert(token_id.clone(), token);
        Promise::new(self.treasury.clone()).transfer(NearToken::from_yoctonear(MINT_PRICE))
    }

    pub fn nft_token(&self, token_id: String) -> Option<serde_json::Value> {
        let token = self.tokens.get(&token_id)?;
        let media = if token.revealed {
            token.metadata.media.clone()
        } else {
            Some(self.hidden_uri.clone())
        };
        Some(serde_json::json!({
            "token_id": token.token_id,
            "owner_id": token.owner_id,
            "metadata": {
                "title": token.metadata.title,
                "description": token.metadata.description,
                "media": media,
            },
            "revealed": token.revealed,
            "sold": token.sold
        }))
    }

    pub fn nft_tokens(&self, from_index: Option<U128>, limit: Option<u64>) -> Vec<serde_json::Value> {
        let start = from_index.map(|v| v.0 as usize).unwrap_or(0);
        let limit = limit.unwrap_or(50) as usize;
        self.token_list.iter()
            .skip(start)
            .take(limit)
            .filter_map(|id| self.nft_token(id.clone()))
            .collect()
    }

    pub fn nft_tokens_for_owner(&self, account_id: AccountId, from_index: Option<U128>, limit: Option<u64>) -> Vec<serde_json::Value> {
        let start = from_index.map(|v| v.0 as usize).unwrap_or(0);
        let limit = limit.unwrap_or(50) as usize;
        self.token_list.iter()
            .filter_map(|id| self.nft_token(id.clone()))
            .filter(|t| t["owner_id"].as_str() == Some(account_id.as_str()))
            .skip(start)
            .take(limit)
            .collect()
    }

    pub fn available_tokens(&self) -> Vec<serde_json::Value> {
        self.token_list.iter()
            .filter_map(|id| {
                let token = self.tokens.get(id)?;
                if !token.sold { self.nft_token(id.clone()) } else { None }
            })
            .collect()
    }

    pub fn nft_total_supply(&self) -> U128 {
        U128(self.token_list.len() as u128)
    }

    pub fn reveal_token(&mut self, token_id: String) {
        self.assert_owner();
        let mut token = self.tokens.get(&token_id).expect("Token not found").clone();
        token.revealed = true;
        self.tokens.insert(token_id, token);
    }

    pub fn reveal_all(&mut self) {
        self.assert_owner();
        let ids: Vec<String> = self.token_list.clone();
        for id in ids {
            if let Some(mut token) = self.tokens.get(&id).cloned() {
                token.revealed = true;
                self.tokens.insert(id, token);
            }
        }
    }

    pub fn set_hidden_uri(&mut self, hidden_uri: String) {
        self.assert_owner();
        self.hidden_uri = hidden_uri;
    }
}
