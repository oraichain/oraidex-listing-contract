use cosmwasm_schema::cw_serde;
use cosmwasm_std::{from_slice, to_vec, Addr, StdError, StdResult, Storage};

#[cw_serde]
pub struct Config {
    /// Owner If None set, contract is frozen.
    pub owner: Addr,
    pub cw20_code_id: u64,
    pub factory_addr: Addr,
}

pub fn config_save(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    Ok(storage.set(KEY_CONFIG, &to_vec(config)?))
}

pub fn config_update(
    storage: &mut dyn Storage,
    owner: Option<Addr>,
    cw20_code_id: Option<u64>,
    factory_addr: Option<Addr>,
) -> StdResult<Config> {
    let mut config = config_read(storage)?;
    let mut need_update = false;

    if let Some(owner) = owner {
        config.owner = owner;
        need_update = true;
    }
    if let Some(cw20_code_id) = cw20_code_id {
        config.cw20_code_id = cw20_code_id;
        need_update = true;
    }
    if let Some(factory_addr) = factory_addr {
        config.factory_addr = factory_addr;
        need_update = true;
    }
    if need_update {
        config_save(storage, &config)?;
    }
    Ok(config)
}

pub fn config_read(storage: &dyn Storage) -> StdResult<Config> {
    match storage.get(KEY_CONFIG) {
        Some(data) => from_slice(&data),
        None => Err(StdError::generic_err("Config not found")),
    }
}

pub static KEY_CONFIG: &[u8] = b"config";
