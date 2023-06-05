#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
    StdResult, SubMsg, SubMsgResult, WasmMsg, Event, SubMsgResponse, 
};
use oraiswap::asset::AssetInfo;
use schemars::_serde_json::to_string_pretty;
use anybuf::Anybuf;
use crate::error::ContractError;
use crate::helpers::FactoryContract;
use crate::msg::{ExecuteMsg, InstantiateMsg, ListTokenMsg, MigrateMsg, QueryMsg};
use crate::state::{
    config_read, config_save, cw20_token_reply_args, cw20_token_reply_args_read,
    listing_token_reply_args, listing_token_reply_args_read, Config, ListingTokenReplyArgs,
};
use cw20_base::msg::InstantiateMsg as TokenInstantiateMsg;
use oraiswap::factory::ExecuteMsg as FactoryExecuteMsg;

/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:oraidex-listing-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

const INSTANTIATE_REPLY_ID: u64 = 1;
const CREATE_PAIR_REPLY_ID: u64 = 2;
const CREATE_PROPOSAL_REPLY_ID: u64 = 3;

pub fn read_response<'a>(key: &str, response: &'a SubMsgResponse) -> StdResult<&'a Event> {
    response
        .events
        .iter()
        .find(|&e| e.ty.eq(key))
        .ok_or_else(|| StdError::generic_err("No event found"))
}

pub fn read_event<'a>(key: &str, event: &'a Event) -> StdResult<&'a str> {
    event
        .attributes
        .iter()
        .find_map(|attr| {
            if attr.key.eq(key) {
                Some(attr.value.as_str())
            } else {
                None
            }
        })
        .ok_or_else(|| StdError::generic_err("No attribute found"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        INSTANTIATE_REPLY_ID => match &reply.result {
            SubMsgResult::Ok(response) => {
                let wasm = read_response("wasm", response)?;
                let cw20_address = Addr::unchecked(read_event("_contract_address", wasm)?);
                
                let config = config_read(deps.storage)?;                
                let listing_contract = FactoryContract(config.factory_addr);
                let create_pair_msg = listing_contract.call(FactoryExecuteMsg::CreatePair {
                    asset_infos: [
                        AssetInfo::NativeToken {
                            denom: "orai".to_string(),
                        },
                        AssetInfo::Token {
                            contract_addr: cw20_address.clone(),
                        },
                    ],
                })?;
                // store cw20 address so we dont have to filter through the list of events
                cw20_token_reply_args(deps.storage, &cw20_address)?;

                Ok(Response::new()
                    .add_submessage(SubMsg::reply_always(create_pair_msg, CREATE_PAIR_REPLY_ID))
                    .add_attributes(vec![
                        ("action", "create_cw20_token_pair"),
                        ("reply_cw20_address", &cw20_address.to_string()),
                    ]))
            }
            SubMsgResult::Err(err) => Err(ContractError::Std(StdError::generic_err(format!(
                "error in instantiate submessage: {}",
                err
            )))),
        },
        CREATE_PAIR_REPLY_ID => match &reply.result {
            SubMsgResult::Ok(response) => {
                let wasm = read_response("wasm", response)?;              
                let lp_address = read_event("liquidity_token_addr", wasm)?;                   
                let reply_args = listing_token_reply_args_read(deps.storage)?;
                let cw20_address = cw20_token_reply_args_read(deps.storage)?;
                // now that we have enough information, we create the proposal
                let text_proposal = Anybuf::new() 
                    .append_string(1, format!(
                        "OraiDEX frontier - Listing new LP mining pool of token {}",
                        cw20_address
                    ))
                    .append_string(2, format!("Create a new liquidity mining pool for CW20 token: {} with LP Address: {}. Reward Assets per second for the liquidity mining pool: {}", cw20_address, lp_address, to_string_pretty(&reply_args.liquidity_pool_reward_assets)?));
                              

                let msg_submit_proposal = Anybuf::new()
                .append_message(1, 
                    &Anybuf::new()
                        .append_string(1, "/cosmos.gov.v1beta1.TextProposal")
                        .append_message(2,&text_proposal)     
                )
                .append_bytes(2, &[])
                .append_bytes(3, env.contract.address.as_bytes());

                let cosmos_msg = CosmosMsg::Stargate {
                    type_url: "/cosmos.gov.v1beta1.MsgSubmitProposal".to_string(),
                    value: msg_submit_proposal.as_bytes().into(),
                };

                Ok(Response::new()
                    .add_attributes(vec![("action", "create_new_token_listing_proposal")])
                    .add_submessage(SubMsg::reply_on_error(cosmos_msg, CREATE_PROPOSAL_REPLY_ID)))
            }
            SubMsgResult::Err(err) => Err(ContractError::Std(StdError::generic_err(format!(
                "error in CREATE_PAIR_REPLY_ID: {}",
                err
            )))),
        },
        CREATE_PROPOSAL_REPLY_ID => match reply.result {
            SubMsgResult::Ok(_) => Ok(Response::new()),
            SubMsgResult::Err(err) => Err(ContractError::Std(StdError::generic_err(format!(
                "error in CREATE_PROPOSAL_REPLY_ID: {}",
                err
            )))),
        },
        _ => Err(ContractError::UnknownReplyId { id: reply.id }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    config_save(
        deps.storage,
        &Config {
            owner: info.sender,
            cw20_code_id: msg.cw20_code_id,
            factory_addr: deps.api.addr_validate(&msg.factory_addr)?,
        },
    )?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ListToken(msg) => list_token(deps, info, msg),
    }
}

pub fn list_token(
    deps: DepsMut,
    info: MessageInfo,
    msg: ListTokenMsg,
) -> Result<Response, ContractError> {
    let config = config_read(deps.storage)?;
    // store liquidity pool reward assets so we can use it later in submsg reply
    listing_token_reply_args(
        deps.storage,
        &ListingTokenReplyArgs {
            proposer: info.sender.to_string(),
            liquidity_pool_reward_assets: msg.liquidity_pool_reward_assets,
        },
    )?;
    let instantiate_msg: CosmosMsg = WasmMsg::Instantiate {
        code_id: config.cw20_code_id,
        funds: vec![],
        admin: None,
        label: msg
            .label
            .unwrap_or(format!("Production Cw20 {} token", msg.symbol.clone())),
        msg: to_binary(&TokenInstantiateMsg {
            name: msg.name.unwrap_or(format!("{} token", msg.symbol.clone())),
            symbol: msg.symbol.clone(),
            decimals: 6,
            initial_balances: msg.initial_balances.unwrap_or_default(),
            mint: msg.mint,
            marketing: msg.marketing,
        })?,
    }
    .into();
    Ok(Response::new()
        .add_submessage(SubMsg::reply_always(instantiate_msg, INSTANTIATE_REPLY_ID))
        .add_attributes(vec![
            ("action", "list_new_cw20_token"),
            ("symbol", &msg.symbol),
        ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&config_read(deps.storage)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    // once we have "migrated", set the new version and return success
    Ok(Response::default())
}

#[cfg(test)]
mod tests {}
