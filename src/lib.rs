mod gable_simulation;
mod structures;

use core::panic;
use std::sync::OnceLock;

use gable_simulation::GableSimulation;
use scrypto_test::ledger_simulator::LedgerSimulatorBuilder;
use substate_store_impls::{rocks_db::RocksdbSubstateStore, substate_database_overlay::UnmergeableSubstateDatabaseOverlay};

fn get_database() -> &'static RocksdbSubstateStore {
    static DATABASE: OnceLock<RocksdbSubstateStore> = OnceLock::new();
    DATABASE.get_or_init(|| {
        const STATE_MANAGER_DATABASE_PATH_ENVIRONMENT_VARIABLE: &str =
            "STATE_MANAGER_DATABASE_PATH";
        let Ok(state_manager_database_path) =
            std::env::var(STATE_MANAGER_DATABASE_PATH_ENVIRONMENT_VARIABLE)
                .map(std::path::PathBuf::from)
        else {
            panic!(
                "The `{}` environment variable is not set",
                STATE_MANAGER_DATABASE_PATH_ENVIRONMENT_VARIABLE
            );
        };
        RocksdbSubstateStore::read_only(state_manager_database_path)
    })
}

pub fn execute_within_environment<'a, F, O>(test_function: F) -> O
where
    F: Fn(GableSimulation<'a>) -> O,
{
    let state_manager = get_database();
    let database =
        UnmergeableSubstateDatabaseOverlay::new_unmergeable(state_manager);
    
    let test_runner: scrypto_test::prelude::LedgerSimulator<scrypto_test::prelude::NoExtension, substate_store_impls::substate_database_overlay::SubstateDatabaseOverlay<&RocksdbSubstateStore, RocksdbSubstateStore>> = LedgerSimulatorBuilder::new()
        .with_custom_database(database)
        .without_kernel_trace()
        .build_without_bootstrapping();
    let simulation = GableSimulation::new(test_runner);
    test_function(simulation)
}
