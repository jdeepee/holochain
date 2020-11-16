use super::{host_fn_api::CallData, install_app, setup_app_inner};
use crate::{
    conductor::{
        api::{CellConductorApi, CellConductorApiT},
        interface::SignalBroadcaster,
        ConductorHandle,
    },
    core::queue_consumer::InitialQueueTriggers,
    core::ribosome::{wasm_ribosome::WasmRibosome, RibosomeT},
};
use holo_hash::{AgentPubKey, DnaHash};
use holochain_keystore::KeystoreSender;
use holochain_p2p::{actor::HolochainP2pRefToCell, HolochainP2pCell};
use holochain_serialized_bytes::SerializedBytes;
use holochain_state::{
    env::EnvironmentWrite,
    test_utils::{test_environments, TestEnvironments},
};
use holochain_types::{
    app::InstalledCell, cell::CellId, dna::DnaDef, dna::DnaFile, test_utils::fake_agent_pubkey_1,
    test_utils::fake_agent_pubkey_2,
};
use holochain_wasm_test_utils::TestWasm;
use holochain_zome_types::zome::ZomeName;
use kitsune_p2p::KitsuneP2pConfig;
use std::{collections::HashMap, convert::TryFrom, sync::Arc};
use tempdir::TempDir;

/// Everything you need to make a call with the host fn api
pub struct ConductorCallData {
    pub cell_id: CellId,
    pub env: EnvironmentWrite,
    pub ribosome: WasmRibosome,
    pub network: HolochainP2pCell,
    pub keystore: KeystoreSender,
    pub signal_tx: SignalBroadcaster,
    pub triggers: InitialQueueTriggers,
    pub cell_conductor_api: CellConductorApi,
}

impl ConductorCallData {
    pub async fn new(cell_id: &CellId, handle: &ConductorHandle, dna_file: &DnaFile) -> Self {
        let env = handle.get_cell_env(cell_id).await.unwrap();
        let keystore = env.keystore().clone();
        let network = handle
            .holochain_p2p()
            .to_cell(cell_id.dna_hash().clone(), cell_id.agent_pubkey().clone());
        let triggers = handle.get_cell_triggers(cell_id).await.unwrap();
        let cell_conductor_api = CellConductorApi::new(handle.clone(), cell_id.clone());

        let ribosome = WasmRibosome::new(dna_file.clone());
        let signal_tx = handle.signal_broadcaster().await;
        ConductorCallData {
            cell_id: cell_id.clone(),
            env,
            ribosome,
            network,
            keystore,
            signal_tx,
            triggers,
            cell_conductor_api,
        }
    }

    /// Create a CallData for a specific zome and call
    pub fn call_data<I: Into<ZomeName>>(&self, zome_name: I) -> CallData {
        let zome_name: ZomeName = zome_name.into();
        let zome_path = (self.cell_id.clone(), zome_name).into();
        let call_zome_handle = self.cell_conductor_api.clone().into_call_zome_handle();
        CallData {
            ribosome: self.ribosome.clone(),
            zome_path,
            network: self.network.clone(),
            keystore: self.keystore.clone(),
            signal_tx: self.signal_tx.clone(),
            call_zome_handle,
        }
    }
}

/// Everything you need to run a test that uses the conductor
pub struct ConductorTestData {
    __tmpdir: Arc<TempDir>,
    // app_api: RealAppInterfaceApi,
    handle: ConductorHandle,
    call_data: HashMap<CellId, ConductorCallData>,
}

impl ConductorTestData {
    pub async fn new(
        envs: TestEnvironments,
        dna_files: Vec<DnaFile>,
        agents: Vec<AgentPubKey>,
        network_config: KitsuneP2pConfig,
    ) -> (Self, HashMap<DnaHash, Vec<CellId>>) {
        let num_agents = agents.len();
        let num_dnas = dna_files.len();
        let mut cells = Vec::with_capacity(num_dnas * num_agents);
        let mut cell_id_by_dna_file = Vec::with_capacity(num_dnas);
        for dna_file in dna_files.iter() {
            let mut cell_ids = Vec::with_capacity(num_agents);
            for (i, agent_id) in agents.iter().enumerate() {
                let cell_id = CellId::new(dna_file.dna_hash().to_owned(), agent_id.clone());
                cells.push((
                    InstalledCell::new(cell_id.clone(), format!("agent-{}", i)),
                    None,
                ));
                cell_ids.push(cell_id);
            }
            cell_id_by_dna_file.push((dna_file, cell_ids));
        }

        let (__tmpdir, _app_api, handle) = setup_app_inner(
            envs,
            vec![("test_app", cells)],
            dna_files.clone(),
            Some(network_config),
        )
        .await;

        let mut call_data = HashMap::new();

        for (dna_file, cell_ids) in cell_id_by_dna_file.iter() {
            for cell_id in cell_ids {
                call_data.insert(
                    cell_id.clone(),
                    ConductorCallData::new(&cell_id, &handle, &dna_file).await,
                );
            }
        }

        let this = Self {
            __tmpdir,
            // app_api,
            handle,
            call_data,
        };
        let installed = cell_id_by_dna_file
            .into_iter()
            .map(|(dna_file, cell_ids)| (dna_file.dna_hash().clone(), cell_ids))
            .collect();
        (this, installed)
    }

    /// Create a new conductor and test data
    pub async fn two_agents(zomes: Vec<TestWasm>, with_bob: bool) -> Self {
        Self::two_agents_inner(zomes, with_bob, None).await
    }

    /// New test data that creates a conductor using a custom network config
    pub async fn with_network_config(
        zomes: Vec<TestWasm>,
        with_bob: bool,
        network: KitsuneP2pConfig,
    ) -> Self {
        Self::two_agents_inner(zomes, with_bob, Some(network)).await
    }

    async fn two_agents_inner(
        zomes: Vec<TestWasm>,
        with_bob: bool,
        network: Option<KitsuneP2pConfig>,
    ) -> Self {
        let dna_file = DnaFile::new(
            DnaDef {
                name: "conductor_test".to_string(),
                uuid: "ba1d046d-ce29-4778-914b-47e6010d2faf".to_string(),
                properties: SerializedBytes::try_from(()).unwrap(),
                zomes: zomes.clone().into_iter().map(Into::into).collect(),
            },
            zomes.into_iter().map(Into::into),
        )
        .await
        .unwrap();

        let mut agents = vec![fake_agent_pubkey_1()];
        if with_bob {
            agents.push(fake_agent_pubkey_2());
        }

        let (this, _) = Self::new(
            test_environments(),
            vec![dna_file],
            agents,
            network.unwrap_or_default(),
        )
        .await;

        this
    }

    /// Shutdown the conductor
    pub async fn shutdown_conductor(&mut self) {
        let shutdown = self.handle.take_shutdown_handle().await.unwrap();
        self.handle.shutdown().await;
        shutdown.await.unwrap();
    }

    /// Bring bob online if he isn't already
    pub async fn bring_bob_online(&mut self) {
        let dna_file = self.alice_call_data().ribosome.dna_file().clone();
        if self.bob_call_data().is_none() {
            let bob_agent_id = fake_agent_pubkey_2();
            let bob_cell_id = CellId::new(dna_file.dna_hash.clone(), bob_agent_id.clone());
            let bob_installed_cell = InstalledCell::new(bob_cell_id.clone(), "bob_handle".into());
            let cell_data = vec![(bob_installed_cell, None)];
            install_app("bob_app", cell_data, vec![dna_file.clone()], self.handle()).await;
            self.call_data.insert(
                bob_cell_id.clone(),
                ConductorCallData::new(&bob_cell_id, &self.handle(), &dna_file).await,
            );
        }
    }

    pub fn handle(&self) -> ConductorHandle {
        self.handle.clone()
    }

    #[allow(clippy::iter_nth_zero)]
    pub fn alice_call_data(&self) -> &ConductorCallData {
        &self.call_data.values().nth(0).unwrap()
    }

    pub fn bob_call_data(&self) -> Option<&ConductorCallData> {
        self.call_data.values().nth(1)
    }

    #[allow(clippy::iter_nth_zero)]
    pub fn alice_call_data_mut(&mut self) -> &mut ConductorCallData {
        let key = self.call_data.keys().nth(0).unwrap().clone();
        self.call_data.get_mut(&key).unwrap()
    }

    pub fn call_data(&mut self, cell_id: &CellId) -> Option<&mut ConductorCallData> {
        self.call_data.get_mut(cell_id)
    }
}
