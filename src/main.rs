use regex::Regex;
use l8kit::{
    L8kitContext,
    types::{EventsTrait}
};

fn main() -> anyhow::Result<()> {
    eprintln!("Starting...");
    l8kit::L8kit::mainnet()
        .from_block(77340040)
        .run(handle_block)
}

async fn handle_block(ctx: L8kitContext) -> anyhow::Result<()> {
    println!("Block {:?}", ctx.block.header().height);
    let re = Regex::new(r"^*.mintbase\d+.near$").unwrap();

    let marketplaces_receipts = ctx.block
        .receipts()
        .iter()
        .filter_map(|executed_receipt| {
            let mint_nft_events: Vec<l8kit::types::Event> = executed_receipt
                .events()
                .iter()
                .cloned()
                .filter(|event| event.event.as_str() == "nft_mint")
                .collect();
            if !mint_nft_events.is_empty() {
                Some((executed_receipt, mint_nft_events))
            } else {
                None
            }
        });

    let mut results: Vec<NFTReceipt> = vec![];
    for (receipt, events) in marketplaces_receipts {
        let marketplace_name = {
            if re.is_match(receipt.receiver_id.as_str()) {
                Marketplace::Mintbase
            } else if receipt.receiver_id.as_str() == "x.paras.near" {
                Marketplace::Paras
            } else {
                Marketplace::Unknown
            }
        };

        results.push(NFTReceipt {
            receipt_id: receipt.receipt_id.to_string(),
            marketplace_name: marketplace_name.name(),
            nfts: events
                .into_iter()
                .filter_map(|event|
                    marketplace_name
                        .convert_event_data_to_nfts(
                            event.data,
                            receipt.receipt_id.to_string()
                        )
                )
                .collect(),
        });
    }
    if !results.is_empty() {
        println!("We caught freshly minted NFTs!\n{:#?}", results);
    }
    Ok(())
}

enum Marketplace {
    Mintbase,
    Paras,
    Unknown,
}

impl Marketplace {
    fn name(&self) -> String {
        match self {
            Self::Mintbase => "Mintbase".to_string(),
            Self::Paras => "Paras".to_string(),
            Self::Unknown => "Unknown".to_string(),
        }
    }
    fn convert_event_data_to_nfts(&self, event_data: Option<serde_json::Value>, receiver_id: String) -> Option<NFT> {
        match self {
            Self::Mintbase => {
                self.mintbase(event_data, receiver_id)
            }
            Self::Paras => {
                self.paras(event_data, receiver_id)
            }
            Self::Unknown => {
                self.unknown(event_data)
            }
        }
    }

    fn paras(&self, event_data: Option<serde_json::Value>, receiver_id: String) -> Option<NFT> {
        if let Some(data) = event_data {
            println!("{:?}", data);
            let paras_event_data = serde_json::from_value::<Vec<ParasEventData>>(data).unwrap();
            Some(NFT {
                owner: paras_event_data[0].owner_id.clone(),
                links: paras_event_data[0]
                    .token_ids
                    .iter()
                    .map(|token_id|
                        format!(
                            "https://paras.id/token/{}::{}/{}",
                            receiver_id,
                            token_id.split(":").collect::<Vec<&str>>()[0],
                            token_id,
                        )
                    )
                    .collect(),
            })
        } else {
            None
        }
    }

    fn mintbase(&self, event_data: Option<serde_json::Value>, receiver_id: String) -> Option<NFT> {
        if let Some(data) = event_data {
            println!("{:#?}", data);
            let mintbase_event_data = serde_json::from_value::<Vec<MintbaseEventData>>(data).unwrap();
            let memo = serde_json::from_str::<MintbaseDataMemo>(&mintbase_event_data[0].memo).unwrap();
            Some(NFT {
                owner: mintbase_event_data[0].owner_id.clone(),
                links: vec![format!("https://mintbase.io/thing/{}:{}", memo.meta_id, receiver_id)],
            })
        } else {
            None
        }
    }

    fn unknown(&self, _event_data: Option<serde_json::Value>) -> Option<NFT> {
        None
    }
}

#[derive(Debug)]
struct NFTReceipt {
    receipt_id: String,
    marketplace_name: String,
    nfts: Vec<NFT>
}

#[derive(Debug)]
struct NFT {
    owner: String,
    links: Vec<String>
}

#[derive(Debug, serde::Deserialize)]
struct ParasEventData {
    owner_id: String,
    token_ids: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct MintbaseEventData {
    owner_id: String,
    memo: String,
}

#[derive(Debug, serde::Deserialize)]
struct MintbaseDataMemo {
    meta_id: String,
    minter: String,
}
