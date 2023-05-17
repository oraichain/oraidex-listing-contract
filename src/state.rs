use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read};
use oraiswap::asset::Asset;

#[cw_serde]
pub struct Config {
    /// Owner If None set, contract is frozen.
    pub owner: Addr,
    pub cw20_code_id: u64,
    pub factory_addr: Addr,
}

pub fn config_save(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
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
    singleton_read(storage, KEY_CONFIG).load()
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
    singleton(storage, KEY_LISTING_TOKEN_REPLY_ARGS).save(reply_args)
}

pub fn listing_token_reply_args_read(storage: &dyn Storage) -> StdResult<ListingTokenReplyArgs> {
    singleton_read(storage, KEY_LISTING_TOKEN_REPLY_ARGS).load()
}

pub fn cw20_token_reply_args(storage: &mut dyn Storage, cw20_address: &Addr) -> StdResult<()> {
    singleton(storage, KEY_CW20_TOKEN_REPLY_ARGS).save(cw20_address)
}

pub fn cw20_token_reply_args_read(storage: &dyn Storage) -> StdResult<Addr> {
    singleton_read(storage, KEY_CW20_TOKEN_REPLY_ARGS).load()
}

pub static KEY_CONFIG: &[u8] = b"config";
pub static KEY_LISTING_TOKEN_REPLY_ARGS: &[u8] = b"listing_token_reply_args";
pub static KEY_CW20_TOKEN_REPLY_ARGS: &[u8] = b"cw20_token_reply_args";
