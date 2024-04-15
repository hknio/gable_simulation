use gable_simulation::*;
use radix_engine_interface::macros::dec;
use sbor::rust::collections::IndexMap;
use scrypto::{math::Decimal, runtime::NonFungibleLocalId};
use rand::seq::SliceRandom;
use csv::Writer;

fn main() {
    execute_within_environment(|mut simulation| {
        // select nft with most lsu to be withdrawn last
        let mut nfts = simulation.get_lsu_claims(false);
        nfts.sort_by(|_, v1, _, v2| v2.0.cmp(&v1.0));
        let nft_with_most_lsu = nfts.first().unwrap();

        println!("NFT with most LSU: {:}, LSU: {}, XRD claim: {}", nft_with_most_lsu.0, nft_with_most_lsu.1.0, nft_with_most_lsu.1.1);

        println!("STEP 1: Recover owner liquidity and use it as validator reward");
        let owner_liqudity = simulation.get_owner_liqudity();
        let pool_liqudity = simulation.get_xrd_balance();
        let owner_xrd_balance = simulation.get_owner_xrd_balance();
        println!("-- Owner liquidity: {}, Pool liquidity: {}, Owner account XRD balance: {}", owner_liqudity, pool_liqudity, owner_xrd_balance);
        let needed_xrd = pool_liqudity - owner_liqudity;
        println!("-- We need {} XRD rewards to recover owner liquidity", needed_xrd);

        let mut csv = csv::Writer::from_path("perfect_simulation.csv").unwrap();
        csv.write_record(&["Day", "LSU locked"]).unwrap();

        let mut day = 0;
        while day < 100 {
            day += 1;
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
            let pool_liqudity = simulation.get_xrd_balance();
            let lsu = simulation.get_lsu_balance();
            println!("-- Day: {}, epoch {}, LSU locked in contract: {}, new validator rewards: {}, pool liqudity: {}", day, epoch.number(), lsu, rewards, pool_liqudity);
            csv.write_record(&[day.to_string(), lsu.to_string()]).unwrap();
            if pool_liqudity > owner_liqudity {
                println!("-- Pool liquidity ({}) is higher than owner liquidity ({}), we can recover owner liquidity", owner_liqudity, pool_liqudity);
                simulation.owner_withdraw_xrd(owner_liqudity);
                break;
            }
        }

        let owner_xrd_balance = simulation.get_owner_xrd_balance();
        let owner_lsu_balance = simulation.get_owner_lsu_balance();
        println!("-- Gable Owner account XRD balance: {}, LSU balance: {}", owner_xrd_balance, owner_lsu_balance);
        let reward_to_add = owner_xrd_balance - 100;
        println!("-- Adding {} XRD rewards to the pool by using Gable Owner account XRD", reward_to_add);
        simulation.stake_xrd_as_owner_and_start_unlock(simulation.gable_owner_account, reward_to_add);

        println!("STEP 2: Recover users LSU and use them as validator reward till");

        'simulation: while day < 360 {
            day += 1;

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
                    println!("-- Claiming XRD reward: {}", reward);
                    simulation.claim_xrd();
                });
            let owner_liqudity = simulation.get_owner_liqudity();
            let mut pool_liqudity = simulation.get_pool_liqudity();
            let lsu = simulation.get_lsu_balance();
            println!("-- Day: {}, epoch {}, LSU locked in contract: {}, new validator rewards: {}, pool liqudity: {}", day, epoch.number(), lsu, rewards, pool_liqudity);
            csv.write_record(&[day.to_string(), lsu.to_string()]).unwrap();

            if pool_liqudity > dec!(0) {
                simulation.update_supplier_kvs(); // recalculates user rewards
                let mut nfts = simulation.get_lsu_claims(true);
                nfts.remove(nft_with_most_lsu.0);
                nfts.sort_by(|_, v1, _, v2| v2.0.cmp(&v1.0));

                if nfts.len() == 0 {
                    println!("No more NFTs to withdraw, all NFTs (except one with the most LSU) have been withdrawn");
                    break 'simulation;
                }

                let mut randomly_selected_nfts = Vec::new();
                for (nft, (lsu, xrd)) in nfts {
                    if pool_liqudity > dec!(0) && xrd <= pool_liqudity + owner_liqudity {
                        randomly_selected_nfts.push((nft, lsu, xrd));
                        pool_liqudity -= xrd;
                    }
                }

                let mut recovered_lsu = dec!(0);
                for (nft, lsu, xrd) in randomly_selected_nfts {
                    println!("-- Withdrawing NFT: {:?} with LSU: {} and XRD claim: {}", nft, lsu, xrd);
                    simulation.create_nft_duplicate(simulation.account, nft.clone());
                    simulation.withdraw_lsu(nft);
                    recovered_lsu += lsu;
                }
                if recovered_lsu > dec!(0) {
                    println!("-- Using recovered {} LSU from NFTs as new validator rewards", recovered_lsu);
                    simulation.stake_lsu_as_owner_and_start_unlock(simulation.account, recovered_lsu);
                }
            }
        }

        csv.flush().unwrap();
    });
}
