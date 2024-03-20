use radix_engine_derive::ScryptoSbor;
use sbor::rust::collections::IndexMap;
use scrypto::{blueprints::resource::Vault, component::KeyValueStore, math::Decimal, resource::ResourceManager, runtime::NonFungibleLocalId, types::ResourceAddress};

#[derive(ScryptoSbor)]
pub struct Flashloanpool {
    // total liquidity vault
    pub liquidity_pool_vault: Vault,
    // validator owner vault
    pub owner_badge_address: ResourceAddress,
    // liquidity that is supplied by the owner
    pub owner_liquidity: Decimal,
    // reference to the admin badge
    pub admin_badge_address: ResourceAddress,
    // index map storing aggregate supplier information
    pub supplier_aggregate_im: IndexMap<u64, Vec<Decimal>>,
    // key value store that stores individual supplier information
    pub supplier_partitioned_kvs: KeyValueStore<u64, IndexMap<NonFungibleLocalId, Vec<Decimal>>>,
    // reference to 'proof of supply nft'
    pub pool_nft: ResourceManager,
    // nft local id number
    pub pool_nft_nr: u64,
    // vault storing supplier's LSU's
    pub lsu_vault: Vault,
    // liquidity that is supplied by staking rewards
    pub rewards_liquidity: Decimal,
    // vault storing the validator owner badge
    pub validator_owner_vault: Vault,
    // vault storing the unstaking lsu's
    pub unstaking_lsu_vault: Vault,
    // vault storing unstaking nft
    pub unstaking_nft_vault: Vault,
    // reference to transient token
    pub transient_token: ResourceManager,
    // interest rate
    pub interest_rate: Decimal,
    // map dize
    pub box_size: u64,
    // ordered nft local id vec
    pub nft_vec: Vec<NonFungibleLocalId>,
}