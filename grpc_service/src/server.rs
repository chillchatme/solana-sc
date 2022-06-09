// use std::os::linux::process;

use tonic::{transport::Server, Request, Response, Status};

use blockchain::blockchain_server::{Blockchain, BlockchainServer};
use blockchain::{CreateUserAccountReq, CreateUserAccountRes};

use chill_cli::app::App;

pub mod blockchain {
    tonic::include_proto!("blockchain");
}


#[derive(Debug, Default)]
pub struct BlockchainServerImpl {}


#[tonic::async_trait]
impl Blockchain for BlockchainServerImpl {
    async fn create_user_account(
        &self,
        _: Request<CreateUserAccountReq>,
    ) -> Result<Response<CreateUserAccountRes>, Status> {
        // println!("Got a request: {:?}", request);

        let app = App::init_from(&["./chill-cli", "create-wallet"]);
        let processed_data = app.run_with_result().map_err(|e| Status::internal(e.to_string()))?;
        
        match processed_data {
            chill_cli::app::ProcessedData::CreateWallet{wallet, signature} => {
                // println!("{} {}", "Wallet:", wallet);
                // println!("{} {}", "Signature:", signature);
                let reply = blockchain::CreateUserAccountRes {
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