#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
    StdResult, SubMsg, SubMsgResult, WasmMsg,
};
use oraiswap::asset::AssetInfo;
use oraiswap::response::MsgInstantiateContractResponse;
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::helpers::FactoryContract;
use crate::msg::{ExecuteMsg, InstantiateMsg, ListTokenMsg, QueryMsg};
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        INSTANTIATE_REPLY_ID => match reply.result {
            SubMsgResult::Ok(response) => {
                let data = response.data.unwrap();
                let res =
                    MsgInstantiateContractResponse::try_from(data.as_slice()).map_err(|_| {
                        StdError::parse_err(
                            "MsgInstantiateContractResponse",
                            "failed to parse MsgInstantiateContractResponse data",
                        )
                    })?;
                let config = config_read(deps.storage)?;

                let cw20_address = Addr::unchecked(res.address);
                let listing_contract: FactoryContract = FactoryContract(config.factory_addr);
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
            SubMsgResult::Err(err) => Err(ContractError::Std(StdError::generic_err(err))),
        },
        CREATE_PAIR_REPLY_ID => match reply.result {
            SubMsgResult::Ok(msg) => {
                let reply_args = listing_token_reply_args_read(deps.storage)?;
                let cw20_address = cw20_token_reply_args_read(deps.storage)?;
                // now that we have enough information, we create the proposal
                // TODO: Need to create a stargate CosmosMsg because CosmWasm does not support submitting a text proposal through smart contracts
                // let cosmos_msg = CosmosMsg::Stargate { type_url: "/cosmos.gov.v1beta1.MsgSubmitProposal".to_string(), value: () }
                Ok(Response::new())
            }
            SubMsgResult::Err(err) => Err(ContractError::Std(StdError::generic_err(err))),
        },
        CREATE_PROPOSAL_REPLY_ID => match reply.result {
            SubMsgResult::Ok(_) => Ok(Response::new()),
            SubMsgResult::Err(err) => Err(ContractError::Std(StdError::generic_err(err))),
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
    _info: MessageInfo,
    msg: ListTokenMsg,
) -> Result<Response, ContractError> {
    let config = config_read(deps.storage)?;
    // store liquidity pool reward assets so we can use it later in submsg reply
    listing_token_reply_args(
        deps.storage,
        &ListingTokenReplyArgs {
            liquidity_pool_reward_assets: msg.liquidity_pool_reward_assets,
        },
    )?;
    Ok(Response::new()
        .add_submessage(SubMsg::reply_always(
            WasmMsg::Instantiate {
                code_id: config.cw20_code_id,
                funds: vec![],
                admin: None,
                label: msg
                    .label
                    .unwrap_or(format!("Production Cw20 {} token", msg.symbol)),
                msg: to_binary(&TokenInstantiateMsg {
                    name: "oraiswap liquidity token".to_string(),
                    symbol: "uLP".to_string(),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: msg.mint,
                    marketing: msg.marketing,
                })?,
            },
            INSTANTIATE_REPLY_ID,
        ))
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

#[cfg(test)]
mod tests {}
