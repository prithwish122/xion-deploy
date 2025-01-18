use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use cw20_base::state::TOKEN_INFO;
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128,
};
use cw20::{BalanceResponse, TokenInfoResponse};
use cw20_base::state::{BALANCES, TokenInfo, MinterData};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let token_info = TokenInfo {
        name: msg.name,
        symbol: msg.symbol,
        decimals: msg.decimals,
        total_supply: Uint128::zero(),
        mint: Some(MinterData {
            minter: info.sender.clone(),
            cap: None,
        }),
    };
    
    TOKEN_INFO.save(deps.storage, &token_info)?;

    for coin in msg.initial_balances {
        let address = deps.api.addr_validate(&coin.address)?;
        BALANCES.save(deps.storage, &address, &coin.amount)?;
    }

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("token_name", token_info.name)
        .add_attribute("token_symbol", token_info.symbol))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Transfer { recipient, amount } => {
            execute_transfer(deps, env, info, recipient, amount)
        }
        ExecuteMsg::Mint { recipient, amount } => {
            execute_mint(deps, env, info, recipient, amount)
        }
        ExecuteMsg::Donate { from, to, amount } => {
            execute_donate(deps, env, info, from, to, amount)
        }
    }
}

pub fn execute_transfer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount == Uint128::zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    let rcpt_addr = deps.api.addr_validate(&recipient)?;
    BALANCES.update(
        deps.storage,
        &info.sender,
        |balance: Option<Uint128>| -> StdResult<_> {
            let balance = balance.unwrap_or_default();
            Ok(balance.checked_sub(amount)?)
        },
    )?;
    BALANCES.update(
        deps.storage,
        &rcpt_addr,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_add(amount)?)
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "transfer")
        .add_attribute("from", info.sender)
        .add_attribute("to", recipient)
        .add_attribute("amount", amount))
}

pub fn execute_mint(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount == Uint128::zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    let token_info = TOKEN_INFO.load(deps.storage)?;
    if token_info.mint.as_ref().map(|m| &m.minter) != Some(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let rcpt_addr = deps.api.addr_validate(&recipient)?;
    BALANCES.update(
        deps.storage,
        &rcpt_addr,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_add(amount)?)
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "mint")
        .add_attribute("to", recipient)
        .add_attribute("amount", amount))
}

pub fn execute_donate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    from: String,
    to: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount == Uint128::zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    let from_addr = deps.api.addr_validate(&from)?;
    let to_addr = deps.api.addr_validate(&to)?;

    let from_balance = BALANCES.load(deps.storage, &from_addr)?;
    if from_balance < amount {
        return Err(ContractError::InsufficientFunds {});
    }

    BALANCES.update(
        deps.storage,
        &from_addr,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    BALANCES.update(
        deps.storage,
        &to_addr,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_add(amount)?)
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "donate")
        .add_attribute("from", from)
        .add_attribute("to", to)
        .add_attribute("amount", amount))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance { address } => query_balance(deps, address),
        QueryMsg::TokenInfo {} => query_token_info(deps),
    }
}

fn query_balance(deps: Deps, address: String) -> StdResult<Binary> {
    let address = deps.api.addr_validate(&address)?;
    let balance = BALANCES.load(deps.storage, &address)?;
    to_binary(&BalanceResponse { balance })
}

fn query_token_info(deps: Deps) -> StdResult<Binary> {
    let info = TOKEN_INFO.load(deps.storage)?;
    to_binary(&TokenInfoResponse {
        name: info.name,
        symbol: info.symbol,
        decimals: info.decimals,
        total_supply: info.total_supply,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            name: "Quest Token".to_string(),
            symbol: "QF".to_string(),
            decimals: 6,
            initial_balances: vec![],
        };
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }
}