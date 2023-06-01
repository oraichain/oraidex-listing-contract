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
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("Got a submessage reply with unknown id: {id}")]
    UnknownReplyId { id: u64 },
}
