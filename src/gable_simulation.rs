
use radix_engine::blueprints::models::FieldPayload;
use radix_engine::blueprints::models::IndexEntryPayload;
use radix_engine::system::system_db_reader::SystemDatabaseWriter;
use radix_engine_interface::macros::dec;
use scrypto::api::ModuleId;
use scrypto::blueprints::resource::FungibleResourceManagerMintInput;
use scrypto::blueprints::resource::FUNGIBLE_RESOURCE_MANAGER_MINT_IDENT;
use scrypto::types::MAIN_BASE_PARTITION;
use scrypto_test::ledger_simulator::LedgerSimulator;
use radix_engine_common::prelude::*;
use substate_store_impls::rocks_db::RocksdbSubstateStore;
use substate_store_interface::db_key_mapper::MappedCommittableSubstateDatabase;
use substate_store_interface::db_key_mapper::SpreadPrefixKeyMapper;
use substate_store_impls::substate_database_overlay::*;
use substate_store_queries::typed_substate_layout::NonFungibleVaultBalanceFieldPayload;
use substate_store_queries::typed_substate_layout::NonFungibleVaultCollection;
use substate_store_queries::typed_substate_layout::NonFungibleVaultField;
use substate_store_queries::typed_substate_layout::NonFungibleVaultNonFungibleEntryPayload;
use substate_store_queries::typed_substate_layout::PartitionDescription;
use substate_store_queries::typed_substate_layout::UnstakeData;
use substate_store_queries::typed_substate_layout::ValidatorStateFieldPayload;
use substate_store_queries::typed_substate_layout::ValidatorStateV1;
use transaction::builder::ManifestBuilder;
use transaction::model::TestTransaction;
use transaction::model::TransactionManifestV1;
use radix_engine_interface::types::CollectionDescriptor;
use radix_engine::system::system_db_reader::SystemDatabaseReader;
use radix_engine::system::system_modules::*;
use radix_engine::transaction::*;
use radix_engine::vm::*;
use radix_engine_interface::blueprints::account::*;
use extend::*;

use crate::structures::Flashloanpool;

pub type GableSimulationTestRunner<'a> = LedgerSimulator<NoExtension, SubstateDatabaseOverlay<&'a RocksdbSubstateStore, RocksdbSubstateStore>>;

pub struct GableSimulation<'a> {
    pub test_runner: GableSimulationTestRunner<'a>,
    pub gable_component: ComponentAddress,
    pub gable_validator: ComponentAddress,
    pub gable_owner_account: ComponentAddress,
    pub gable_owner_badge: ResourceAddress,
    pub validator_owner_badge: NonFungibleLocalId,
    pub lsu: ResourceAddress,
    pub pool_nft: ResourceAddress,
    pub account: ComponentAddress,
}

impl<'a> GableSimulation<'a> {
    pub fn new(mut test_runner: GableSimulationTestRunner<'a>) -> Self {        
        let decoder = AddressBech32Decoder::new(&NetworkDefinition::mainnet());
        let gable_component = ComponentAddress::try_from_bech32(&decoder, "component_rdx1cpmh7lyg0hx6efv5q79lv6rqxdqpuh27y99nzm0jpwu2u44ne243ws").unwrap();
        let gable_validator = ComponentAddress::try_from_bech32(&decoder, "validator_rdx1sdf04wxuc7c4llwst8rw5sfj350gnlnluhrpy09wk2gwk5cmvgffpy").unwrap();
        let gable_owner_account = ComponentAddress::try_from_bech32(&decoder, "account_rdx12y0frj9mjxmlc36gggsts826jsp2e6wk40tv85prpve32kx2u360y3").unwrap();
        let gable_owner_badge: ResourceAddress = ResourceAddress::try_from_bech32(&decoder, "resource_rdx1t4zd2h95htm79dmyr9d422qy4c03urvkutqxgsyxx9udcmrdgk9s22").unwrap();
        let validator_owner_badge: NonFungibleLocalId = NonFungibleLocalId::from_str("[8352fab8dcc7b15ffdd059c6ea41328d1e89fe7fe5c6123caeb290eb531b]").unwrap();
        let lsu: ResourceAddress = ResourceAddress::try_from_bech32(&decoder, "resource_rdx1thrz4g8g83802lumrtrdsrhjd6k5uxhxhgkrwjg0jn75cvxfc99nap").unwrap();
        let gable_state : Flashloanpool = test_runner.component_state(gable_component);
        let pool_nft = gable_state.pool_nft.address();
        let account = test_runner.new_account_with_xrd();
        GableSimulation {
            test_runner,
            gable_component,
            gable_validator,
            gable_owner_account,
            gable_owner_badge,
            validator_owner_badge,
            lsu,
            pool_nft,
            account
        }
    }

    pub fn gable_state(&mut self) -> Flashloanpool {
        self.test_runner.component_state(self.gable_component)
    }

    pub fn validator_state(&mut self) -> ValidatorStateV1 {
        self.test_runner.component_state::<ValidatorStateFieldPayload>(self.gable_validator).into_latest()
    }

    pub fn get_supplier_partitioned_kvs(&mut self) -> IndexMap<u64, IndexMap<NonFungibleLocalId, Vec<Decimal>>> {
        let mut ret : IndexMap<u64, IndexMap<NonFungibleLocalId, Vec<Decimal>>> = IndexMap::new();
        let kv_node_id = self.gable_state().supplier_partitioned_kvs.id.as_node_id().clone();
        let reader = SystemDatabaseReader::new(self.test_runner.substate_db());
        reader
            .key_value_store_iter(
                &kv_node_id,
                None,
            )
            .unwrap()
            .collect::<Vec<_>>()
            .iter()
            .for_each(|(k, v)| {
                let k : u64 = scrypto_decode(k).unwrap();
                let v : IndexMap<NonFungibleLocalId, Vec<Decimal>> = scrypto_decode(v).unwrap();
                ret.insert(k, v);
            });
        ret
    }
    
    pub fn get_lsu_claims(&mut self, skip_one_nft_in_each_group: bool) -> IndexMap<NonFungibleLocalId, (Decimal, Decimal)> {
        let nft_groups = self.get_supplier_partitioned_kvs();
        let mut nfts: IndexMap<NonFungibleLocalId, (Decimal, Decimal)> = IndexMap::new();
        for (_group, mut group_nfts) in nft_groups {
            if skip_one_nft_in_each_group {
                group_nfts.sort_by(|_k1, v1, _k2, v2| v2[0].cmp(&v1[0]));
                group_nfts.pop(); // remove one nft with lowest LSU from each group to avoid group deletion issue
            }
            for (nft, amounts) in group_nfts {
                nfts.insert(nft, (amounts[0], amounts[1] + amounts[2]));
            }
        }
        nfts
    }

    pub fn create_nft_duplicate(&mut self, account: ComponentAddress, nft: NonFungibleLocalId) {
        self.test_runner.execute_manifest_without_auth(ManifestBuilder::new()
            .lock_fee(self.account, dec!(10))
            .take_from_worktop(self.pool_nft, 0, "bucket")
            .with_bucket("bucket", |builder, bucket| {
                builder.call_method(
                    account,
                    "deposit",
                    (bucket,),
                )
            })
            .build()
        ).expect_commit_success();

        let vault = self.test_runner.get_component_vaults(account, self.pool_nft)[0];

        let db = self.test_runner.substate_db_mut();
        let reader = SystemDatabaseReader::new(db);
        let vault_balance: NonFungibleVaultBalanceFieldPayload = reader
            .read_typed_object_field(
                &vault,
                ModuleId::Main,
                NonFungibleVaultField::Balance.into(),
            )
            .unwrap();

        let mut vault_balance = vault_balance.into_latest();
        vault_balance.amount += 1;

        let blueprint_id = reader.get_blueprint_id(&vault, ModuleId::Main).unwrap();
        let definition = reader.get_blueprint_definition(&blueprint_id).unwrap();
        let partition_description = &definition
            .interface
            .state
            .get_partition(NonFungibleVaultCollection::NonFungibleIndex.collection_index())
            .unwrap()
            .0;
        let partition_number = match partition_description {
            PartitionDescription::Logical(offset) => {
                MAIN_BASE_PARTITION.at_offset(*offset).unwrap()
            }
            PartitionDescription::Physical(partition_number) => *partition_number,
        };

        let mut writer = SystemDatabaseWriter::new(db);
        writer
            .write_typed_object_field(
                &vault,
                ModuleId::Main,
                NonFungibleVaultField::Balance.into(),
                NonFungibleVaultBalanceFieldPayload::from_content_source(vault_balance),
            )
            .unwrap();
            
        db.put_mapped::<SpreadPrefixKeyMapper, _>(
            &vault,
            partition_number,
            &SubstateKey::Map(nft.to_key()),
            &NonFungibleVaultNonFungibleEntryPayload::from_content_source(()),
        );
    }

    pub fn add_epoch(&mut self, epochs_to_add: u64) {
        let current_epoch = self.test_runner.get_current_epoch();
        self.test_runner.set_current_epoch(current_epoch.after(epochs_to_add).unwrap());
    }

    pub fn get_pending_owner_unlocks(&mut self) -> BTreeMap<Epoch, Decimal> {
        let validator_state: ValidatorStateV1 = self.validator_state();
        validator_state.pending_owner_stake_unit_withdrawals
    }

    pub fn get_pending_unstakes(&mut self) -> BTreeMap<Epoch, Decimal> {        
        let gable_state = self.gable_state();
        let validator_state: ValidatorStateV1 = self.validator_state();
        let mut ret = BTreeMap::new();
        for nft in &gable_state.nft_vec {
            let nft_data: UnstakeData = self.test_runner.get_non_fungible_data(validator_state.claim_nft, nft.clone());
            ret.insert(nft_data.claim_epoch, nft_data.claim_amount);
        }
        ret
    }

    pub fn get_xrd_balance(&mut self) -> Decimal {
        self.test_runner.get_component_balance(self.gable_component, XRD)
    }

    pub fn get_lsu_balance(&mut self) -> Decimal {
        self.test_runner.get_component_balance(self.gable_component, self.lsu)
    }

    pub fn get_owner_liqudity(&mut self) -> Decimal {
        self.gable_state().owner_liquidity
    }

    pub fn get_pool_liqudity(&mut self) -> Decimal {
        self.get_xrd_balance() - self.get_owner_liqudity()
    }

    pub fn finish_unlock_and_unstake(&mut self) {
        self.test_runner.execute_manifest_without_auth(ManifestBuilder::new()
            .lock_fee(self.account, dec!(10))
            .create_proof_from_account_of_amount(self.gable_owner_account, self.gable_owner_badge, dec!(1))
            .call_method(self.gable_component, "finish_unlock_owner_stake_units", (self.gable_validator, self.validator_owner_badge.clone()))
            .call_method(self.gable_component, "unstake", (self.gable_validator,))
            .build()
        ).expect_commit_success();
    }

    pub fn unstake(&mut self) {
        self.test_runner.execute_manifest_without_auth(ManifestBuilder::new()
            .lock_fee(self.account, dec!(10))
            .create_proof_from_account_of_amount(self.gable_owner_account, self.gable_owner_badge, dec!(1))
            .call_method(self.gable_component, "claim_xrd", (self.gable_validator,))
            .build()
        ).expect_commit_success();
    }

    pub fn update_supplier_kvs(&mut self) {
        let supplier_aggregate_im = self.gable_state().supplier_aggregate_im;
        let mut builder = ManifestBuilder::new()
            .lock_fee(self.account, dec!(10))
            .create_proof_from_account_of_amount(self.gable_owner_account, self.gable_owner_badge, dec!(1));
        for (group, balances) in supplier_aggregate_im {
            builder = builder.call_method(self.gable_component, "update_supplier_kvs", (group,));
        }
        self.test_runner.execute_manifest_without_auth(
            builder.build()
        ).expect_commit_success();
    }

    pub fn claim_xrd(&mut self) {
        self.test_runner.execute_manifest_without_auth(ManifestBuilder::new()
            .lock_fee(self.account, dec!(10))
            .create_proof_from_account_of_amount(self.gable_owner_account, self.gable_owner_badge, dec!(1))
            .call_method(self.gable_component, "claim_xrd", (self.gable_validator,))
            .build()
        ).expect_commit_success();
    }

    pub fn withdraw_lsu(&mut self, nft: NonFungibleLocalId) {
        self.test_runner.execute_manifest_without_auth(ManifestBuilder::new()
            .lock_fee(self.account, dec!(10))
            .withdraw_non_fungibles_from_account(self.account, self.pool_nft, vec![nft])
            .take_all_from_worktop(self.pool_nft, "nfts")
            .with_bucket("nfts", |builder, nfts| {
                builder.call_method(
                    self.gable_component,
                    "withdraw_lsu",
                    (nfts,),
                )
            })
            .try_deposit_entire_worktop_or_abort(self.account, None)
            .build()
        ).expect_commit_success();
    }

    pub fn add_validator_reward(&mut self, amount: Decimal) {
        self.test_runner.execute_manifest_without_auth(ManifestBuilder::new()
            .lock_fee(self.account, dec!(10))
            .withdraw_from_account(self.account, XRD, amount)
            .take_all_from_worktop(XRD, "xrd")
            .with_name_lookup(|builder, name_lookup| {
                builder.call_method(
                    self.gable_validator,
                    "stake_as_owner",
                    (name_lookup.bucket("xrd"),),
                )
            })
            .take_all_from_worktop(self.lsu, "lsu")
            .with_name_lookup(|builder, name_lookup| {
                builder.call_method(
                    self.gable_validator,
                    "lock_owner_stake_units",
                    (name_lookup.bucket("lsu"),),
                )
            })            
            .call_method(self.gable_component, "start_unlock_owner_stake_units", (amount, self.gable_validator, self.validator_owner_badge.clone()))
            .build()
        ).expect_commit_success();
    }


}

#[ext]
pub impl<'a> GableSimulationTestRunner<'a> {
    fn execute_manifest_without_auth(
        &mut self,
        manifest: TransactionManifestV1,
    ) -> TransactionReceiptV1 {
        self.execute_manifest_with_enabled_modules(
            manifest,
            EnabledModules::for_notarized_transaction() & !EnabledModules::AUTH,
        )
    }

    fn execute_manifest_with_enabled_modules(
        &mut self,
        manifest: TransactionManifestV1,
        enabled_modules: EnabledModules,
    ) -> TransactionReceiptV1 {
        let mut execution_config = ExecutionConfig::for_notarized_transaction(
            NetworkDefinition::mainnet(),
        );
        execution_config.enabled_modules = enabled_modules;
        let nonce = self.next_transaction_nonce();
        let test_transaction = TestTransaction::new_from_nonce(manifest, nonce);
        let prepared_transaction = test_transaction.prepare().unwrap();
        let executable =
            prepared_transaction.get_executable(Default::default());
        self.execute_transaction(
            executable,
            Default::default(),
            execution_config,
        )
    }

    fn new_account_with_xrd(&mut self) -> ComponentAddress {
        let (public_key, _private_key) = self.new_key_pair();
        let test_account_address =
            ComponentAddress::virtual_account_from_public_key(
                &public_key
            );
        self.execute_manifest_with_enabled_modules(
            ManifestBuilder::new()
                .call_method(XRD, FUNGIBLE_RESOURCE_MANAGER_MINT_IDENT, FungibleResourceManagerMintInput {
                    amount: dec!(100_000_000),
                }).call_method(test_account_address, ACCOUNT_DEPOSIT_BATCH_IDENT, (ManifestExpression::EntireWorktop,)).build(),
            EnabledModules::for_notarized_transaction() & !EnabledModules::AUTH & !EnabledModules::COSTING,
        ).expect_commit_success();
    
        test_account_address
    }
}