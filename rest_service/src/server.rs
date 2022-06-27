use chill_cli::app::App;
use chill_cli::cli::{RPC_URL, MINT, ACCOUNT, PAYER, PRIMARY_WALLET, PROGRAM_ID};

use axum::{
    routing::{get, post},
    http::StatusCode,
    response::IntoResponse,
    Json, Router
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;

pub const RESTAPI_PORT_ENV: &str = "RESTAPI_PORT";
pub const RESTAPI_PORT_DEFAULT: u16 = 3000;

fn get_port() -> u16 {
    match std::env::var(RESTAPI_PORT_ENV) {
        Ok(val) => {
            let parsed_val = val.parse::<u16>();
            match parsed_val {
                Ok(port) => {
                    return port;
                },
                Err(parse_e) => {
                    println!("{RESTAPI_PORT_ENV} parse error {:?}", parse_e);
                }
            }
        },
        Err(_) => {
            println!("{RESTAPI_PORT_ENV} wasn't set");
        },
    }

    println!("use default port {:?}", RESTAPI_PORT_DEFAULT);
    RESTAPI_PORT_DEFAULT
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(root))
        .route("/balance", post(balance))
        .route("/info", post(info))
        .route("/create-wallet", post(create_wallet));

    let addr = SocketAddr::from(([127, 0, 0, 1], get_port()));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root() -> &'static str {
    "Rest full blockchain server is working!"
}

async fn balance(
    Json(balance_req): Json<BalanceReq>,
) -> impl IntoResponse {

    let mut args: String = "./chill-cli balance".into();

    if !balance_req.url.is_empty() {
        args.push_str(&format!(" --{} {}", RPC_URL, balance_req.url));
    }
    if !balance_req.mint_address.is_empty() {
        args.push_str(&format!(" --{} {}", MINT, balance_req.mint_address));
    }
    if !balance_req.account.is_empty() {
        args.push_str(&format!(" --{} {}", ACCOUNT, balance_req.account));
    }

    let args = args.split_whitespace().collect::<Vec<&str>>();

    let app_init_result = App::init_from_save(&args);
    if let Err(e) = app_init_result {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response();
    }
    let processed_data_result = app_init_result.unwrap().run_with_result();
    match processed_data_result {
        Ok(chill_cli::app::ProcessedData::Balance(balance)) =>
            return (StatusCode::OK, Json(BalanceRes { balance })).into_response(),
        Ok(_) =>
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "wrong processed data"}))).into_response(),
        Err(e) => 
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    };
}

async fn info(
    Json(info_req): Json<InfoReq>,
) -> impl IntoResponse {

    let mut args: String = "./chill-cli info".into();

    if !info_req.url.is_empty() {
        args.push_str(&format!(" --{} {}", RPC_URL, info_req.url));
    }
    if !info_req.mint_address.is_empty() {
        args.push_str(&format!(" --{} {}", MINT, info_req.mint_address));
    }

    let args = args.split_whitespace().collect::<Vec<&str>>();

    let app_init_result = App::init_from_save(&args);
    if let Err(e) = app_init_result {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response();
    }
    let processed_data_result = app_init_result.unwrap().run_with_result();
    match processed_data_result {
        Ok(chill_cli::app::ProcessedData::Info(info)) =>
            return (StatusCode::OK, Json(InfoRes { info })).into_response(),
        Ok(_) =>
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "wrong processed data"}))).into_response(),
        Err(e) => 
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    };

}


async fn create_wallet(
    Json(create_wallet_req): Json<CreateWalletReq>,
) -> impl IntoResponse {

    let mut args: String = "./chill-cli create-wallet".into();

    if !create_wallet_req.url.is_empty() {
        args.push_str(&format!(" --{} {}", RPC_URL, create_wallet_req.url));
    }
    if !create_wallet_req.account.is_empty() {
        args.push_str(&format!(" --{} {}", ACCOUNT, create_wallet_req.account));
    }
    if !create_wallet_req.payer.is_empty() {
        args.push_str(&format!(" --{} {}", PAYER, create_wallet_req.payer));
    }
    if !create_wallet_req.primary_wallet.is_empty() {
        args.push_str(&format!(" --{} {}", PRIMARY_WALLET, create_wallet_req.primary_wallet));
    }
    if !create_wallet_req.program_id.is_empty() {
        args.push_str(&format!(" --{} {}", PROGRAM_ID, create_wallet_req.program_id));
    }

    let args = args.split_whitespace().collect::<Vec<&str>>();

    let app_init_result = App::init_from_save(&args);
    if let Err(e) = app_init_result {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response();
    }
    let processed_data_result = app_init_result.unwrap().run_with_result();
    match processed_data_result {
        Ok(chill_cli::app::ProcessedData::CreateWallet{wallet, signature}) =>
            return (StatusCode::OK,
                    Json(CreateWalletRes { wallet: wallet.to_string(), signature: signature.to_string()})).into_response(),
        Ok(_) =>
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "wrong processed data"}))).into_response(),
        Err(e) => 
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    };
}



#[derive(Deserialize)]
struct BalanceReq {
    url: String,
    mint_address: String,
    account: String,    
}

#[derive(Serialize)]
struct BalanceRes {
    balance: f64,
}

#[derive(Deserialize)]
struct InfoReq {
    url: String,
    mint_address: String,
}

#[derive(Serialize)]
struct InfoRes {
    info: String,
}

#[derive(Deserialize)]
struct CreateWalletReq {
    url: String,
    account: String,
    payer: String,
    primary_wallet: String,
    program_id: String,
}

#[derive(Serialize)]
struct CreateWalletRes {
    wallet: String,
    signature: String,
}