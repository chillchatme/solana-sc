// use std::os::linux::process;

use tonic::{transport::Server, Request, Response, Status};

use blockchain::blockchain_server::{Blockchain, BlockchainServer};
use blockchain::{BalanceReq, BalanceRes, InfoReq, InfoRes, CreateWalletReq, CreateWalletRes};

use chill_cli::app::App;
use chill_cli::cli::{RPC_URL, MINT, ACCOUNT, PAYER, PRIMARY_WALLET, PROGRAM_ID};

pub mod blockchain {
    tonic::include_proto!("blockchain");
}


#[derive(Debug, Default)]
pub struct BlockchainServerImpl {}


#[tonic::async_trait]
impl Blockchain for BlockchainServerImpl {
    async fn balance(
        &self,
        balance_req: Request<BalanceReq>,
    ) -> Result<Response<BalanceRes>, Status> {
        
        let BalanceReq {
            url,
            mint_address,
            account
        } = &balance_req.into_inner();
        

        let mut args: String = "./chill-cli balance".into();
        
        if !url.is_empty() {
            args.push_str(&format!(" --{} {}", RPC_URL, url));
        }
        if !mint_address.is_empty() {
            args.push_str(&format!(" --{} {}", MINT, mint_address));
        }
        if !account.is_empty() {
            args.push_str(&format!(" --{} {}", ACCOUNT, account));
        }

        let args = args.split_whitespace().collect::<Vec<&str>>();

        let app = App::init_from(&args);
        let processed_data = app.run_with_result().map_err(|e| Status::internal(e.to_string()))?;
     
        match processed_data {
            chill_cli::app::ProcessedData::Balance(balance) => {
                let reply = blockchain::BalanceRes {
                    balance,
                };
                return Ok(Response::new(reply));
            },
            _ => return Err(Status::internal("balance internal error")),
        };     
    }

    async fn info(
        &self,
        info_req: Request<InfoReq>,
    ) -> Result<Response<InfoRes>, Status> {

        let InfoReq {
            url,
            mint_address
        } = &info_req.into_inner();

        let mut args: String = "./chill-cli info".into();

        if !url.is_empty() {
            args.push_str(&format!(" --{} {}", RPC_URL, url));
        }
        if !mint_address.is_empty() {
            args.push_str(&format!(" --{} {}", MINT, mint_address));
        }

        let args = args.split_whitespace().collect::<Vec<&str>>();

        let app = App::init_from(&args);
        let processed_data = app.run_with_result().map_err(|e| Status::internal(e.to_string()))?;
     
        match processed_data {
            chill_cli::app::ProcessedData::Info(info) => {
                let reply = blockchain::InfoRes {
                    info,
                };
                return Ok(Response::new(reply));
            },
            _ => return Err(Status::internal("info internal error")),
        };     
    }

    async fn create_wallet(
        &self,
        create_wallet_req: Request<CreateWalletReq>,
    ) -> Result<Response<CreateWalletRes>, Status> {

        let CreateWalletReq {
            url,
            account,
            payer,
            primary_wallet,
            program_id
        } = &create_wallet_req.into_inner();

        let mut args: String = "./chill-cli create-wallet".into();

        if !url.is_empty() {
            args.push_str(&format!(" --{} {}", RPC_URL, url));
        }
        if !account.is_empty() {
            args.push_str(&format!(" --{} {}", ACCOUNT, account));
        }
        if !payer.is_empty() {
            args.push_str(&format!(" --{} {}", PAYER, payer));
        }
        if !primary_wallet.is_empty() {
            args.push_str(&format!(" --{} {}", PRIMARY_WALLET, primary_wallet));
        }
        if !program_id.is_empty() {
            args.push_str(&format!(" --{} {}", PROGRAM_ID, program_id));
        }

        let args = args.split_whitespace().collect::<Vec<&str>>();

        let app = App::init_from(&args);
        let processed_data = app.run_with_result().map_err(|e| Status::internal(e.to_string()))?;
        
        match processed_data {
            chill_cli::app::ProcessedData::CreateWallet{wallet, signature} => {
                let reply = blockchain::CreateWalletRes {
                    wallet: wallet.to_string(),
                    signature: signature.to_string(),
                };
                return Ok(Response::new(reply));
            },
            _ => return Err(Status::internal("create-wallet internal error")),
        };        
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let blockchain_server = BlockchainServerImpl::default();

    Server::builder()
        .add_service(BlockchainServer::new(blockchain_server))
        .serve(addr)
        .await?;

    Ok(())
}