use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    // #[error("{0}")]
    // ProtobufError(#[from] protobuf::Error),
    #[error("{0}")]
    JsonError(#[from] schemars::_serde_json::Error),

    #[error("Unauthorized")]
    Unauthorized {},
}
