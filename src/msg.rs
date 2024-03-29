use cosmwasm_schema::{cw_serde, QueryResponses};
use cw20::{Cw20Coin, MinterResponse};
use cw20_base::msg::InstantiateMarketingInfo;
use oraiswap::asset::{Asset, AssetInfo};

#[cw_serde]
pub struct InstantiateMsg {
    pub cw20_code_id: u64,
    pub factory_addr: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    ListToken(ListTokenMsg),
}

#[cw_serde]
pub struct ListTokenMsg {
    pub targeted_asset_info: Option<AssetInfo>,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub initial_balances: Option<Vec<Cw20Coin>>,
    pub cw20_admin: Option<String>,
    pub liquidity_pool_reward_assets: Vec<Asset>,
    pub label: Option<String>,
    pub mint: Option<MinterResponse>,
    pub marketing: Option<InstantiateMarketingInfo>,
    pub pair_asset_info: AssetInfo,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(crate::state::Config)]
    Config {},
}

#[cw_serde]
pub struct MigrateMsg {}
