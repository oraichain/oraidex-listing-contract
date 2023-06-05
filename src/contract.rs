use crate::error::ContractError;
use crate::helpers::FactoryContract;
use crate::msg::{ExecuteMsg, InstantiateMsg, ListTokenMsg, MigrateMsg, QueryMsg};
use crate::state::{config_read, config_save, Config};
use anybuf::Anybuf;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
    StdResult, SubMsg, SubMsgResponse, SubMsgResult, WasmMsg,
};
use cw20_base::msg::InstantiateMsg as TokenInstantiateMsg;
use oraiswap::asset::AssetInfo;
use oraiswap::factory::ExecuteMsg as FactoryExecuteMsg;

/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:oraidex-listing-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

const INSTANTIATE_REPLY_ID: u64 = 1;
const CREATE_PAIR_REPLY_ID: u64 = 2;
const CREATE_PROPOSAL_REPLY_ID: u64 = 3;

pub fn read_attr<'a>(key: &str, response: &'a SubMsgResponse) -> StdResult<&'a str> {
    for event in &response.events {
        if let Some(attr) = event.attributes.iter().find_map(|attr| {
            if attr.key.eq(key) {
                Some(attr.value.as_str())
            } else {
                None
            }
        }) {
            return Ok(attr);
        }
    }

    Err(StdError::generic_err("No attribute found"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> StdResult<Response> {
    match &reply.result {
        SubMsgResult::Ok(response) => match reply.id {
            INSTANTIATE_REPLY_ID => {
                let cw20_address = Addr::unchecked(read_attr("_contract_address", response)?);
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

                Ok(Response::new()
                    .add_submessage(SubMsg::reply_always(create_pair_msg, CREATE_PAIR_REPLY_ID)))
            }
            CREATE_PAIR_REPLY_ID => {
                let lp_address = read_attr("liquidity_token_addr", response)?;
                let cw20_address = read_attr("pair", response)?
                    .splitn(2, '-')
                    .last()
                    .ok_or_else(|| StdError::generic_err("No attribute found"))?;

                // now that we have enough information, we create the proposal
                let text_proposal = Anybuf::new() .append_string(1, format!(
                            "OraiDEX frontier - Listing new LP mining pool of token {}",
                            cw20_address
                        ))
                        .append_string(2, format!("Create a new liquidity mining pool for CW20 token: {} with LP Address: {}", cw20_address, lp_address,                     
                    ));

                let msg_submit_proposal = Anybuf::new()
                    .append_message(
                        1,
                        &Anybuf::new()
                            .append_string(1, "/cosmos.gov.v1beta1.TextProposal")
                            .append_message(2, &text_proposal),
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
            CREATE_PROPOSAL_REPLY_ID => Ok(Response::new()),
            _ => Err(StdError::generic_err(format!(
                "reply (id {:?}) invalid",
                reply.id
            ))),
        },
        SubMsgResult::Err(err) => match reply.id {
            INSTANTIATE_REPLY_ID => Err(StdError::generic_err(format!(
                "error in instantiate submessage: {}",
                err
            ))),
            CREATE_PAIR_REPLY_ID => Err(StdError::generic_err(format!(
                "error in CREATE_PAIR_REPLY_ID: {}",
                err
            ))),
            _ => Err(StdError::generic_err(format!(
                "reply (id {:?}) error {:?}",
                reply.id, err
            ))),
        },
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
            ("proposer", info.sender.as_str()),
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
