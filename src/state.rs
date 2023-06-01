use cosmwasm_schema::cw_serde;
use cosmwasm_std::{from_slice, to_vec, Addr, StdError, StdResult, Storage};
use oraiswap::asset::Asset;

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

#[cw_serde]
pub struct ListingTokenReplyArgs {
    pub proposer: String,
    pub liquidity_pool_reward_assets: Vec<Asset>,
}

pub fn listing_token_reply_args(
    storage: &mut dyn Storage,
    reply_args: &ListingTokenReplyArgs,
) -> StdResult<()> {
    Ok(storage.set(KEY_LISTING_TOKEN_REPLY_ARGS, &to_vec(reply_args)?))
}

pub fn listing_token_reply_args_read(storage: &dyn Storage) -> StdResult<ListingTokenReplyArgs> {
    match storage.get(KEY_LISTING_TOKEN_REPLY_ARGS) {
        Some(data) => from_slice(&data),
        None => Err(StdError::generic_err("Listing token not found")),
    }
}

pub fn cw20_token_reply_args(storage: &mut dyn Storage, cw20_address: &Addr) -> StdResult<()> {
    Ok(storage.set(KEY_CW20_TOKEN_REPLY_ARGS, &to_vec(cw20_address)?))
}

pub fn cw20_token_reply_args_read(storage: &dyn Storage) -> StdResult<Addr> {
    match storage.get(KEY_CW20_TOKEN_REPLY_ARGS) {
        Some(data) => from_slice(&data),
        None => Err(StdError::generic_err("Cw20 token not found")),
    }
}

pub static KEY_CONFIG: &[u8] = b"config";
pub static KEY_LISTING_TOKEN_REPLY_ARGS: &[u8] = b"listing_token_reply_args";
pub static KEY_CW20_TOKEN_REPLY_ARGS: &[u8] = b"cw20_token_reply_args";
