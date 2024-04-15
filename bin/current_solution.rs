use gable_simulation::*;
use radix_engine_interface::macros::dec;
use sbor::rust::collections::IndexMap;
use scrypto::{math::Decimal, runtime::NonFungibleLocalId};
use rand::seq::SliceRandom;
use csv::Writer;

fn main() {
    execute_within_environment(|mut simulation| {
        // select top 3 nf's with most lsu to be withdrawn last
        let mut nfts = simulation.get_lsu_claims(false);
        nfts.sort_by(|_, v1, _, v2| v2.0.cmp(&v1.0));
        let three_nfts_with_most_lsu = nfts.into_iter().take(3).collect::<Vec<_>>();

        println!("Three NFTs with most LSU: ");
        for (nft, (lsu, xrd)) in &three_nfts_with_most_lsu {
            println!("-- NFT: {:?} with LSU: {} and XRD claim: {}", nft, lsu, xrd);
        }
        
        let mut csv = csv::Writer::from_path("current_simulation.csv").unwrap();
        csv.write_record(&["Day", "LSU locked"]).unwrap();

        // simulating 360 days of rewards
        'simulation: for day in 1..=360 {
            // add reward from validator and move epoch (time) by 24h
            simulation.add_validator_reward(dec!(8000));
            simulation.add_epoch(288);

            let epoch = simulation.test_runner.get_current_epoch();
            simulation.get_pending_owner_unlocks().iter().filter(|unlock| unlock.0 <= &epoch).for_each(|_| {
                simulation.finish_unlock_and_unstake();
            });
            let mut rewards = dec!(0);
            simulation.get_pending_unstakes()
                .iter()
                .filter(|unlock| unlock.0 <= epoch)
                .for_each(|(epoch, reward)| {
                    rewards += *reward;
                    simulation.claim_xrd();
                });
            let owner_liqudity = simulation.get_owner_liqudity();
            let mut pool_liqudity = simulation.get_pool_liqudity();
            let lsu = simulation.get_lsu_balance();
            println!("Day: {}, epoch {}, LSU locked in contract: {}, new validator rewards: {}, pool liqudity: {}", day, epoch.number(), lsu, rewards, pool_liqudity);
            csv.write_record(&[day.to_string(), lsu.to_string()]).unwrap();

            if pool_liqudity > dec!(0) {
                simulation.update_supplier_kvs(); // recalculates user rewards
                let mut nfts = simulation.get_lsu_claims(true);
                for nft_to_remove in &three_nfts_with_most_lsu {
                    nfts.remove(&nft_to_remove.0);
                }

                if nfts.len() == 0 {
                    println!("No more NFTs to withdraw, all NFTs (except top three) have been withdrawn");
                    break 'simulation;
                }

                let mut random_nfts = nfts.into_iter().collect::<Vec<_>>();
                random_nfts.shuffle(&mut rand::thread_rng());

                let mut randomly_selected_nfts = Vec::new();
                for (nft, (lsu, xrd)) in random_nfts {
                    if pool_liqudity > dec!(0) && xrd <= pool_liqudity + owner_liqudity {
                        randomly_selected_nfts.push((nft, lsu, xrd));
                        pool_liqudity -= xrd;
                    }
                }

                for (nft, lsu, xrd) in randomly_selected_nfts {
                    println!("-- Withdrawing NFT: {:?} with LSU: {} and XRD claim: {}", nft, lsu, xrd);
                    simulation.create_nft_duplicate(simulation.account, nft.clone());
                    simulation.withdraw_lsu(nft);
                }
            }
        }

        csv.flush().unwrap();
    });
}
