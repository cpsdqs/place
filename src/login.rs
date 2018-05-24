use crypto_hash::{hex_digest, Algorithm};
use serde::{Serialize, Serializer};
use serde_json;
use std::collections::{BTreeMap, HashMap};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Logins {
    #[serde(serialize_with = "ser_hash_map", flatten)]
    logins: HashMap<String, Login>,
}

impl Logins {
    pub fn init() -> Logins {
        match fs::read("logins.json") {
            Ok(file) => match serde_json::from_slice(&file) {
                Ok(logins) => logins,
                Err(err) => panic!("Failed to read logins.json: {:?}", err),
            },
            Err(_) => Logins {
                logins: HashMap::new(),
            },
        }
    }

    pub fn verify_login(&self, login: &str, password: &str) -> bool {
        match self.logins.get(login) {
            Some(login) => login.verify(password),
            None => false,
        }
    }
}

fn ser_hash_map<S>(value: &HashMap<String, Login>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let map: BTreeMap<_, _> = value.iter().collect();
    map.serialize(serializer)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Login {
    salt: String,
    digest: String,
}

impl Login {
    pub fn verify(&self, password: &str) -> bool {
        let bytes: Vec<u8> = (password.to_string() + &self.salt).bytes().collect();
        hex_digest(Algorithm::SHA256, &bytes) == self.digest
    }
}
