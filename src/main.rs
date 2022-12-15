use regex::Regex;
// use near_lake_framework::near_lake_primitives::types::events::EventsTrait;

fn main() -> anyhow::Result<()> {
    eprintln!("Starting...");
    // Lake Framework start boilerplate
    near_lake_framework::LakeBuilder::default()
        .mainnet()
        .start_block_height(80504433)
        .build()
        .expect("Failed to build Lake")
        .run(handle_block) // developer-defined async function that handles each block
}

async fn handle_block(mut ctx: near_lake_framework::LakeContext) -> anyhow::Result<()> {
    println!("Block {:?}", ctx.block.header().height);
    let re = Regex::new(r"^*.mintbase\d+.near$").unwrap();

    // Indexing lines START
    let nfts: Vec<NFTReceipt> = ctx
        .events() // getting all the events happened in the block
        .iter()
        .filter(|(_receipt_id, event)| event.event.as_str() == "nft_mint") // filter them by "nft_mint" event only
        .filter_map(|(receipt_id, event)| {
            // Next we're parsing the event to catch Marketplaces we know (Mintbase and Paras)
            // then we parse the event_data to extract: owner, link to the NFT
            // collect all the NFTs (excluding Marketplaces or contracts we don't know how to parse)
            // and print data about caught NFTS to the terminal
            // NB! The logic on the next lines DOES NOT relate to Lake Framework!
            let receipt = &ctx.action_by_receipt_id(receipt_id)
                .expect("Expect `ActionReceipt` to be included in the block");
            let marketplace = {
                if re.is_match(receipt.receiver_id.as_str()) {
                    Marketplace::Mintbase
                } else if receipt.receiver_id.as_str() == "x.paras.near" {
                    Marketplace::Paras
                } else {
                    Marketplace::Unknown
                }
            };

            if let Some(nft) = marketplace.convert_event_data_to_nfts(
                event.clone().data,
                receipt.receiver_id.to_string(),
            ) {
                Some(NFTReceipt {
                    receipt_id: receipt.receipt_id.to_string(),
                    marketplace_name: marketplace.name(),
                    nfts: vec![nft],
                })
            } else {
                None
            }
        })
        .collect();
    // Indexing lines END

    if !nfts.is_empty() {
        println!("We caught freshly minted NFTs!\n{:#?}", nfts);
    }
    Ok(())
}

// Next lines are just defined structures and methods to support
// our indexing goal: catch NFT MINT events and print links
// to newly created NFTS
// Next lines have nothing to do about the NEAR Lake Framework
// It is developer-defined logic for indexing their needs

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
            let mintbase_event_data = serde_json::from_value::<Vec<MintbaseEventData>>(data).unwrap();
            Some(NFT {
                owner: mintbase_event_data[0].owner_id.clone(),
                links: vec![format!("https://mintbase.io/contract/{}/token/{}", receiver_id, mintbase_event_data[0].token_ids[0])],
            })
        } else {
            None
        }
    }

    fn unknown(&self, _event_data: Option<serde_json::Value>) -> Option<NFT> {
        None
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct NFTReceipt {
    receipt_id: String,
    marketplace_name: String,
    nfts: Vec<NFT>
}

#[allow(dead_code)]
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
    token_ids: Vec<String>,
}
