#[cfg(not(feature = "library"))]
use crate::error::ContractError;
use crate::helpers::FactoryContract;
use crate::msg::{ExecuteMsg, InstantiateMsg, ListTokenMsg, MigrateMsg, QueryMsg};
use crate::state::{config_read, config_save, pair_asset_info_read, pair_asset_info_save, Config};
use cosmos_sdk_proto::{
    cosmos::gov::v1beta1::{MsgSubmitProposal, TextProposal},
    traits::Message,
    Any,
};

use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsg, SubMsgResponse, SubMsgResult, WasmMsg,
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
                let create_pair_msg = create_pair_msg(
                    config.factory_addr,
                    [
                        pair_asset_info_read(deps.storage)?,
                        AssetInfo::Token {
                            contract_addr: cw20_address.clone(),
                        },
                    ],
                )?;

                Ok(Response::new()
                    .add_message(create_pair_msg)
                    /*
                    .add_submessage(SubMsg::reply_on_error(
                        create_pair_msg,
                        CREATE_PAIR_REPLY_ID,
                    ))
                     */
                    .add_attribute("cw20_address", cw20_address))
            }
            CREATE_PAIR_REPLY_ID => {
                let lp_address = read_attr("liquidity_token_addr", response)?;
                let cw20_address = read_attr("pair", response)?
                    .splitn(2, '-')
                    .last()
                    .ok_or_else(|| StdError::generic_err("No attribute found"))?;

                let cosmos_msg = CosmosMsg::Stargate {
                    type_url: "/cosmos.gov.v1beta1.MsgSubmitProposal".to_string(),
                    value: MsgSubmitProposal {
                        content: Some(Any {
                            type_url: "/cosmos.gov.v1beta1.TextProposal".to_string(),
                            value:TextProposal {
                                title:format!(
                                    "OraiDEX frontier - Listing new LP mining pool of token {}",
                                    cw20_address
                                ),
                                description:format!("Create a new liquidity mining pool for CW20 token: {} with LP Address: {}", cw20_address, lp_address,
                            )
                            }.encode_to_vec()
                        }),
                        initial_deposit: vec![],
                        proposer: env.contract.address.to_string(),
                    }.encode_to_vec().into(),
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

pub fn create_pair_msg(factory_addr: Addr, asset_infos: [AssetInfo; 2]) -> StdResult<CosmosMsg> {
    let listing_contract = FactoryContract(factory_addr.clone());
    listing_contract.call(FactoryExecuteMsg::CreatePair {
        asset_infos,
        pair_admin: Some(factory_addr.into_string()),
    })
}

pub fn list_token(
    deps: DepsMut,
    info: MessageInfo,
    msg: ListTokenMsg,
) -> Result<Response, ContractError> {
    let config = config_read(deps.storage)?;
    if let Some(asset_info) = msg.targeted_asset_info {
        let cosmos_msg = create_pair_msg(
            config.factory_addr,
            [asset_info.clone(), msg.pair_asset_info],
        )?;
        return Ok(Response::new()
            // .add_submessage(SubMsg::reply_always(cosmos_msg, CREATE_PAIR_REPLY_ID))
            .add_message(cosmos_msg)
            .add_attributes(vec![
                ("action", "list_new_token_pool"),
                ("proposer", info.sender.as_str()),
                ("asset_info", &asset_info.to_string()),
            ]));
    }

    if msg.symbol.is_none() {
        return Err(ContractError::Std(StdError::generic_err(
            "CW20 token symbol is empty. Cannot create a new CW20 token for pool listing",
        )));
    }
    let symbol = msg.symbol.unwrap();

    let instantiate_msg: CosmosMsg = WasmMsg::Instantiate {
        code_id: config.cw20_code_id,
        funds: vec![],
        admin: msg.cw20_admin,
        label: msg
            .label
            .unwrap_or(format!("Production Cw20 {} token", symbol.clone())),
        msg: to_binary(&TokenInstantiateMsg {
            name: msg.name.unwrap_or(format!("{} token", symbol.clone())),
            symbol: symbol.clone(),
            decimals: 6,
            initial_balances: msg.initial_balances.unwrap_or_default(),
            mint: msg.mint,
            marketing: msg.marketing,
        })?,
    }
    .into();
    pair_asset_info_save(deps.storage, &msg.pair_asset_info)?;
    Ok(Response::new()
        .add_submessage(SubMsg::reply_always(instantiate_msg, INSTANTIATE_REPLY_ID))
        .add_attributes(vec![
            ("action", "list_new_token_pool_with_new_cw20_token"),
            ("symbol", &symbol),
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
mod tests {
    use anybuf::Anybuf;
    use cosmos_sdk_proto::{
        cosmos::gov::v1beta1::{MsgSubmitProposal, TextProposal},
        traits::Message,
        Any,
    };
    use cosmwasm_std::Binary;

    #[test]
    fn test_encode() {
        let cw20_address = "orai123";

        let value: Binary = MsgSubmitProposal {
            content: Some(Any {
                type_url: "/cosmos.gov.v1beta1.TextProposal".to_string(),
                value: TextProposal {
                    title: format!(
                        "OraiDEX frontier - Listing new LP mining pool of token {}",
                        cw20_address
                    ),
                    description: format!(
                        "Create a new liquidity mining pool for CW20 token: {} with LP Address: {}",
                        cw20_address, cw20_address,
                    ),
                }
                .encode_to_vec(),
            }),
            initial_deposit: vec![],
            proposer: cw20_address.to_string(),
        }
        .encode_to_vec()
        .into();

        // now that we have enough information, we create the proposal
        let text_proposal = Anybuf::new()
            .append_string(
                1,
                format!(
                    "OraiDEX frontier - Listing new LP mining pool of token {}",
                    cw20_address
                ),
            )
            .append_string(
                2,
                format!(
                    "Create a new liquidity mining pool for CW20 token: {} with LP Address: {}",
                    cw20_address, cw20_address,
                ),
            );

        let msg_submit_proposal = Anybuf::new()
            .append_message(
                1,
                &Anybuf::new()
                    .append_string(1, "/cosmos.gov.v1beta1.TextProposal")
                    .append_message(2, &text_proposal),
            )
            .append_bytes(2, &[])
            .append_bytes(3, cw20_address.as_bytes());
        let value2: Binary = msg_submit_proposal.as_bytes().into();

        assert_eq!(value.to_base64(), value2.to_base64());
    }
}
