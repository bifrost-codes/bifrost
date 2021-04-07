// Copyright 2019-2021 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

use hex_literal::hex;
use sc_chain_spec::ChainType;
use sp_core::{crypto::UncheckedInto, sr25519};
use telemetry::TelemetryEndpoints;
use node_primitives::AccountId;
use pallet_im_online::sr25519::{AuthorityId as ImOnlineId};
use bifrost_runtime::{
	constants::currency::DOLLARS,
	AuthorityDiscoveryConfig, BabeConfig, BalancesConfig,
	CouncilConfig, DemocracyConfig, ImOnlineConfig, PoaManagerConfig,
	GenesisConfig, GrandpaConfig, IndicesConfig, SessionConfig, SessionKeys,
	SudoConfig, SystemConfig, TechnicalCommitteeConfig, VestingConfig,
	WASM_BINARY, wasm_binary_unwrap
};
use crate::chain_spec::{
	Extensions, BabeId, GrandpaId, AuthorityDiscoveryId,
	authority_keys_from_seed, get_account_id_from_seed, testnet_accounts
};

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const DEFAULT_PROTOCOL_ID: &str = "bnc";

/// The `ChainSpec` parametrised for the bifrost runtime.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

pub fn config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../../res/bifrost-poa.json")[..])
}

fn session_keys(
	grandpa: GrandpaId,
	babe: BabeId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
	SessionKeys {
		babe,
		grandpa,
		im_online,
		authority_discovery,
	}
}

fn staging_testnet_config_genesis() -> GenesisConfig {
	// stash, controller, session-key
	// generated with secret:
	// for i in 1 2 3 4 ; do for j in stash controller; do subkey inspect "$secret"/fir/$j/$i; done; done
	// and
	// for i in 1 2 3 4 ; do for j in session; do subkey --ed25519 inspect "$secret"//fir//$j//$i; done; done

	let initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId)> = vec![(
		// 5Fbsd6WXDGiLTxunqeK5BATNiocfCqu9bS1yArVjCgeBLkVy
		hex!["9c7a2ee14e565db0c69f78c7b4cd839fbf52b607d867e9e9c5a79042898a0d12"].into(),
		// 5EnCiV7wSHeNhjW3FSUwiJNkcc2SBkPLn5Nj93FmbLtBjQUq
		hex!["781ead1e2fa9ccb74b44c19d29cb2a7a4b5be3972927ae98cd3877523976a276"].into(),
		// 5Fb9ayurnxnaXj56CjmyQLBiadfRCqUbL2VWNbbe1nZU6wiC
		hex!["9becad03e6dcac03cee07edebca5475314861492cdfc96a2144a67bbe9699332"].unchecked_into(),
		// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
		hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"].unchecked_into(),
		// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
		hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"].unchecked_into(),
		// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
		hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"].unchecked_into(),
	),(
		// 5ERawXCzCWkjVq3xz1W5KGNtVx2VdefvZ62Bw1FEuZW4Vny2
		hex!["68655684472b743e456907b398d3a44c113f189e56d1bbfd55e889e295dfde78"].into(),
		// 5Gc4vr42hH1uDZc93Nayk5G7i687bAQdHHc9unLuyeawHipF
		hex!["c8dc79e36b29395413399edaec3e20fcca7205fb19776ed8ddb25d6f427ec40e"].into(),
		// 5EockCXN6YkiNCDjpqqnbcqd4ad35nU4RmA1ikM4YeRN4WcE
		hex!["7932cff431e748892fa48e10c63c17d30f80ca42e4de3921e641249cd7fa3c2f"].unchecked_into(),
		// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
		hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"].unchecked_into(),
		// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
		hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"].unchecked_into(),
		// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
		hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"].unchecked_into(),
	),(
		// 5DyVtKWPidondEu8iHZgi6Ffv9yrJJ1NDNLom3X9cTDi98qp
		hex!["547ff0ab649283a7ae01dbc2eb73932eba2fb09075e9485ff369082a2ff38d65"].into(),
		// 5FeD54vGVNpFX3PndHPXJ2MDakc462vBCD5mgtWRnWYCpZU9
		hex!["9e42241d7cd91d001773b0b616d523dd80e13c6c2cab860b1234ef1b9ffc1526"].into(),
		// 5E1jLYfLdUQKrFrtqoKgFrRvxM3oQPMbf6DfcsrugZZ5Bn8d
		hex!["5633b70b80a6c8bb16270f82cca6d56b27ed7b76c8fd5af2986a25a4788ce440"].unchecked_into(),
		// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
		hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"].unchecked_into(),
		// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
		hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"].unchecked_into(),
		// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
		hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"].unchecked_into(),
	),(
		// 5HYZnKWe5FVZQ33ZRJK1rG3WaLMztxWrrNDb1JRwaHHVWyP9
		hex!["f26cdb14b5aec7b2789fd5ca80f979cef3761897ae1f37ffb3e154cbcc1c2663"].into(),
		// 5EPQdAQ39WQNLCRjWsCk5jErsCitHiY5ZmjfWzzbXDoAoYbn
		hex!["66bc1e5d275da50b72b15de072a2468a5ad414919ca9054d2695767cf650012f"].into(),
		// 5DMa31Hd5u1dwoRKgC4uvqyrdK45RHv3CpwvpUC1EzuwDit4
		hex!["3919132b851ef0fd2dae42a7e734fe547af5a6b809006100f48944d7fae8e8ef"].unchecked_into(),
		// 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
		hex!["00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378"].unchecked_into(),
		// 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
		hex!["00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378"].unchecked_into(),
		// 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
		hex!["00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378"].unchecked_into(),
	)];

	// generated with secret: subkey inspect "$secret"/fir
	let root_key: AccountId = hex![
		// 5Ff3iXP75ruzroPWRP2FYBHWnmGGBSb63857BgnzCoXNxfPo
		"9ee5e5bdc0ec239eb164f865ecc345ce4c88e76ee002e0f7e318097347471809"
	].into();

	let endowed_accounts: Vec<AccountId> = vec![root_key.clone()];

	testnet_genesis(
		initial_authorities,
		root_key,
		Some(endowed_accounts),
	)
}

pub fn staging_testnet_config() -> ChainSpec {
	let boot_nodes = vec![];
	ChainSpec::from_genesis(
		"Bifrost Staging Testnet",
		"bifrost_staging_testnet",
		ChainType::Live,
		staging_testnet_config_genesis,
		boot_nodes,
		Some(TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
			.expect("Staging telemetry url is valid; qed")),
		None,
		None,
		Default::default(),
	)
}

/// Helper function to create bifrost GenesisConfig for testing
pub fn testnet_genesis(
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		GrandpaId,
		BabeId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)>,
	root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
) -> GenesisConfig {
	let mut endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(testnet_accounts);
	let num_endowed_accounts = endowed_accounts.len();

	initial_authorities.iter().for_each(|x|
		if !endowed_accounts.contains(&x.0) {
			endowed_accounts.push(x.0.clone())
		}
	);

	const ENDOWMENT: u128 = 1_000_000 * DOLLARS;

	GenesisConfig {
		frame_system: SystemConfig {
			code: wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		},
		pallet_balances: pallet_balances_config(&endowed_accounts, ENDOWMENT),
		pallet_indices: IndicesConfig {
			indices: vec![],
		},
		pallet_session: SessionConfig {
			keys: initial_authorities.iter().map(|x| {
				(x.0.clone(), x.0.clone(), session_keys(
					x.2.clone(),
					x.3.clone(),
					x.4.clone(),
					x.5.clone(),
				))
			}).collect::<Vec<_>>(),
		},
		pallet_im_online: ImOnlineConfig {
			keys: vec![],
		},
		pallet_democracy: DemocracyConfig::default(),
		pallet_collective_Instance1: CouncilConfig::default(),
		pallet_collective_Instance2: TechnicalCommitteeConfig {
			members: endowed_accounts.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.collect(),
			phantom: Default::default(),
		},
		pallet_sudo: SudoConfig {
			key: root_key.clone(),
		},
		pallet_babe: BabeConfig {
			authorities: vec![],
		},
		pallet_authority_discovery: AuthorityDiscoveryConfig {
			keys: vec![],
		},
		pallet_grandpa: GrandpaConfig {
			authorities: vec![],
		},
		pallet_membership_Instance1: Default::default(),
		pallet_treasury: Default::default(),
		pallet_vesting: VestingConfig {
			vesting: endowed_accounts
				.iter()
				.map(|account_id| (account_id.clone(), 0, 10000, ENDOWMENT / 2))
				.collect::<Vec<_>>()
		},
		brml_poa_manager: PoaManagerConfig {
			initial_validators: initial_authorities
				.iter()
				.map(|x| x.0.clone())
				.collect::<Vec<_>>(),
		},
	}
}

fn development_config_genesis(_wasm_binary: &[u8]) -> GenesisConfig {
	testnet_genesis(
		vec![authority_keys_from_seed("Alice")],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

/// Bifrost development config (single validator Alice)
pub fn development_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or("Bifrost development wasm not available")?;

	let properties = {
		let mut props = serde_json::Map::new();

		props.insert(
			"ss58Format".to_owned(),
			serde_json::value::to_value(6u8).expect("The ss58Format cannot be convert to json value.")
		);
		props.insert(
			"tokenDecimals".to_owned(),
			serde_json::value::to_value(12u8).expect("The tokenDecimals cannot be convert to json value.")
		);
		props.insert(
			"tokenSymbol".to_owned(),
			serde_json::value::to_value("BNC".to_owned()).expect("The tokenSymbol cannot be convert to json value.")
		);
		Some(props)
	};
	let protocol_id = Some("bifrost");

	Ok(ChainSpec::from_genesis(
		"Development",
		"dev",
		ChainType::Development,
		move || development_config_genesis(wasm_binary),
		vec![],
		None,
		protocol_id,
		properties,
		Default::default(),
	))
}

fn local_testnet_genesis(_wasm_binary: &[u8]) -> GenesisConfig {
	testnet_genesis(
		vec![
			authority_keys_from_seed("Alice"),
		],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

/// Bifrost local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or("Bifrost development wasm not available")?;

	Ok(ChainSpec::from_genesis(
		"Bifrost Local Testnet",
		"bifrost_local_testnet",
		ChainType::Local,
		move || local_testnet_genesis(wasm_binary),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		Default::default(),
	))
}

pub fn chainspec_config() -> ChainSpec {
	let properties = {
		let mut props = serde_json::Map::new();

		props.insert(
			"ss58Format".to_owned(),
			serde_json::value::to_value(6u8).expect("The ss58Format cannot be convert to json value.")
		);
		props.insert(
			"tokenDecimals".to_owned(),
			serde_json::value::to_value(12u8).expect("The tokenDecimals cannot be convert to json value.")
		);
		props.insert(
			"tokenSymbol".to_owned(),
			serde_json::value::to_value("BNC".to_owned()).expect("The tokenSymbol cannot be convert to json value.")
		);
		Some(props)
	};
	let protocol_id = Some("bifrost");

	ChainSpec::from_genesis(
		"Bifrost POA Network",
		"bifrost_poa",
		ChainType::Custom("Live".into()),
		bifrost_poa_testnet_config_genesis,
		vec![
			"/ip4/124.156.222.64/tcp/30333/p2p/12D3KooWKstUr5Jd4qBcY1DkTJPwbbeoSbaXHePpB1eGgdxWQ1iS".parse().expect("failed to parse multiaddress."),
			"/ip4/124.156.222.64/tcp/30334/p2p/12D3KooWQ3FWGrqQPkWJdqEWCpLHALiisqKhmnJ1a42DmNuGX3Px".parse().expect("failed to parse multiaddress."),
		],
		Some(TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
			.expect("Bifrost POA Network telemetry url is valid; qed")),
		protocol_id,
		properties,
		Default::default(),
	)
}

pub fn bifrost_poa_testnet_genesis(
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		GrandpaId,
		BabeId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)>,
	root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
) -> GenesisConfig {
	let mut endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(testnet_accounts);

	initial_authorities.iter().for_each(|x|
		if !endowed_accounts.contains(&x.0) {
			endowed_accounts.push(x.0.clone())
		}
	);

	const ENDOWMENT: u128 = 1_000 * DOLLARS;

	// TechnicalCommittee and Council members
	let technicals_member = vec![
		hex!["a8f3801d8c78e090c201eebd56a952a7e90d535ced5684fdd3fd676a9acf4b44"].into(), // lurpis
		hex!["72a7f127d3e7fc34bb22a4262ac6bcc7e35616e4eff43d22da121d57d456fb01"].into(), // edwin
		hex!["0c825c6b5a5223828fa627f34c2fc6290b0e833bcc3ca5de9ca534933add5455"].into(), // jamie
	];

	GenesisConfig {
		frame_system: SystemConfig {
			code: wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		},
		pallet_balances: pallet_balances_config(&technicals_member, ENDOWMENT),
		pallet_indices: IndicesConfig {
			indices: vec![],
		},
		pallet_session: SessionConfig {
			keys: initial_authorities.iter().map(|x| {
				(x.0.clone(), x.0.clone(), session_keys(
					x.2.clone(),
					x.3.clone(),
					x.4.clone(),
					x.5.clone(),
				))
			}).collect::<Vec<_>>(),
		},
		pallet_im_online: ImOnlineConfig {
			keys: vec![],
		},
		pallet_democracy: DemocracyConfig::default(),
		pallet_collective_Instance1: CouncilConfig {
			members: technicals_member.clone(),
			phantom: Default::default(),
		},
		pallet_collective_Instance2: TechnicalCommitteeConfig {
			members: technicals_member,
			phantom: Default::default(),
		},
		pallet_sudo: SudoConfig {
			key: root_key.clone(),
		},
		pallet_babe: BabeConfig {
			authorities: vec![],
		},
		pallet_authority_discovery: AuthorityDiscoveryConfig {
			keys: vec![],
		},
		pallet_grandpa: GrandpaConfig {
			authorities: vec![],
		},
		pallet_membership_Instance1: Default::default(),
		pallet_treasury: Default::default(),
		// pallet_vesting: VestingConfig {
		// 	vesting: endowed_accounts
		// 		.iter()
		// 		.map(|account_id| (account_id.clone(), 0, 10000, ENDOWMENT / 2))
		// 		.collect::<Vec<_>>()
		// },
		pallet_vesting: Default::default(),
		brml_poa_manager: PoaManagerConfig {
			initial_validators: initial_authorities
				.iter()
				.map(|x| x.0.clone())
				.collect::<Vec<_>>(),
		},
	}
}

fn bifrost_poa_testnet_config_genesis() -> GenesisConfig {
	let initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId)> = vec![(
		// 14pXP1DJVJnZDt6CcPkuwfT6NHh9nQ22J8LnsUCvkk2LRH65
		hex!["a8f3801d8c78e090c201eebd56a952a7e90d535ced5684fdd3fd676a9acf4b44"].into(),
		// 14pXP1DJVJnZDt6CcPkuwfT6NHh9nQ22J8LnsUCvkk2LRH65
		hex!["a8f3801d8c78e090c201eebd56a952a7e90d535ced5684fdd3fd676a9acf4b44"].into(),
		// gHq6Rjb3NpwtsQX5HyampguzvaQSsjwdYJmK4Up1oVhVdXH
		hex!["ca02edb17553a26a7a9bf068e32db3d746663b1ecda9fa812e3a5d68001b91fd"].unchecked_into(),
		// cuC2dFwwrLcPQckxadSqqKo8qpPuG5uyHaFf9NXkaLQbdE7
		hex!["340a0e3583eb8056e429747964d43533f280a55f6c33004216d18e0dc5ac6d01"].unchecked_into(),
		// cuC2dFwwrLcPQckxadSqqKo8qpPuG5uyHaFf9NXkaLQbdE7
		hex!["340a0e3583eb8056e429747964d43533f280a55f6c33004216d18e0dc5ac6d01"].unchecked_into(),
		// cuC2dFwwrLcPQckxadSqqKo8qpPuG5uyHaFf9NXkaLQbdE7
		hex!["340a0e3583eb8056e429747964d43533f280a55f6c33004216d18e0dc5ac6d01"].unchecked_into(),
	),(
		// eKHu97Aiwx14PDZ37hvJUScGaLjPUsrALqQVTjg2NET2XsM
		hex!["72a7f127d3e7fc34bb22a4262ac6bcc7e35616e4eff43d22da121d57d456fb01"].into(),
		// eKHu97Aiwx14PDZ37hvJUScGaLjPUsrALqQVTjg2NET2XsM
		hex!["72a7f127d3e7fc34bb22a4262ac6bcc7e35616e4eff43d22da121d57d456fb01"].into(),
		// fnD1YhgjxScs6HZBa6qMai12CwJ7DQogtzBuypqmYCwhKSG
		hex!["b36b577bd97eb7ef54b85a8ddea35ed159ff5121d1c0bbd4394f0b0464e982bf"].unchecked_into(),
		// hJTSFK9rSs4d8wr1N8VhYAMtHkpcA4hYi66Q8Wdg9vMRvub
		hex!["f6b97fc5a3332bfcd532a86b3a4fc31dd91d25aa918915ef97f58a1576d40d56"].unchecked_into(),
		// hJTSFK9rSs4d8wr1N8VhYAMtHkpcA4hYi66Q8Wdg9vMRvub
		hex!["f6b97fc5a3332bfcd532a86b3a4fc31dd91d25aa918915ef97f58a1576d40d56"].unchecked_into(),
		// hJTSFK9rSs4d8wr1N8VhYAMtHkpcA4hYi66Q8Wdg9vMRvub
		hex!["f6b97fc5a3332bfcd532a86b3a4fc31dd91d25aa918915ef97f58a1576d40d56"].unchecked_into(),
	)];

	// generated with secret: subkey inspect "$secret"/fir
	let root_key: AccountId = hex![
		// fGfKaGohG9exHPgUGw2stJphwrGBiePqGLsYTozx6WBz45f
		"9ce28cb8857475c0518b291174746e2db2829b3b61f2cd06d91e485e6f1a023f"
	].into();

	let endowed_accounts: Vec<AccountId> = vec![root_key.clone()];

	bifrost_poa_testnet_genesis(
		initial_authorities,
		root_key,
		Some(endowed_accounts),
	)
}

fn pallet_balances_config(endowed_accounts: &Vec<AccountId>, amount: u128) -> BalancesConfig {
	if let Some(voucher_accounts) = super::initialize_all_vouchers() {
		BalancesConfig {
			balances: voucher_accounts.iter().cloned().filter_map(|voucher| {
				if let Some(who) = endowed_accounts.iter().cloned().find(|x| x == &voucher.0) {
					Some((who, amount))
				} else {
					Some(voucher)
				}
			}).collect()
		}
	} else {
		BalancesConfig {
			balances: endowed_accounts.iter().cloned()
				.map(|x| (x, amount))
				.collect()
		}
	}
}
