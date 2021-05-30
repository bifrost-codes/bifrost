
// Copyright 2019-2021 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

//! The Bifrost Node runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use sp_api::impl_runtime_apis;
use sp_core::{OpaqueMetadata, u32_trait::{_1, _2, _3, _4}};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{BlakeTwo256, Block as BlockT, Zero, UniqueSaturatedInto},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult,DispatchError,DispatchResult,SaturatedConversion,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use core::convert::TryInto;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime, parameter_types, match_type, PalletId,
	traits::{Randomness, IsInVec, All},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
		DispatchClass, IdentityFee, Weight,
	},
	StorageValue,
};
use frame_system::limits::{BlockLength, BlockWeights};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_std::marker::PhantomData;

/// Constant values used within the runtime.
pub mod constants;
use constants::{currency::*, time::*};
use node_primitives::{Moment, Amount, CurrencyId, TokenSymbol};

// XCM imports
use polkadot_parachain::primitives::Sibling;
use xcm::v0::{BodyId, Junction::*, MultiAsset, MultiLocation, MultiLocation::*, NetworkId, Xcm};
use xcm_builder::{
    AccountId32Aliases, AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, CurrencyAdapter,
    EnsureXcmOrigin, FixedWeightBounds, IsConcrete, LocationInverter, NativeAsset,
    ParentAsSuperuser, ParentIsDefault, RelayChainAsNative, SiblingParachainAsNative,
    SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
    SovereignSignedViaLocation, TakeWeightCredit, UsingComponents,
};
use xcm_executor::{Config, XcmExecutor};
use pallet_xcm::XcmPassthrough;
use frame_system::{EnsureRoot, EnsureOneOf};

// orml imports
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::MultiCurrency;

// zenlink imports
use zenlink_protocol::{ZenlinkMultiAssets, LocalAssetHandler, MultiAssetsHandler, make_x2_location, AssetId, AssetBalance, PairInfo};

mod weights;

pub type SessionHandlers = ();

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("bifrost-parachain"),
	impl_name: create_runtime_str!("bifrost-parachain"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

/// We assume that ~10% of the block weight is consumed by `on_initalize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2 seconds of compute with a 6 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = 2 * WEIGHT_PER_SECOND;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub const Version: RuntimeVersion = VERSION;
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub const SS58Prefix: u8 = 6;
}

impl frame_system::Config for Runtime {
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = Indices;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type Event = Event;
	/// The ubiquitous origin type.
	type Origin = Origin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Runtime version.
	type Version = Version;
	/// Converts a module to an index of this module in the runtime.
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = RocksDbWeight;
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = RuntimeBlockLength;
	type SS58Prefix = SS58Prefix;
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
}

parameter_types! {
	pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 1 * MILLIBNC;
	pub const TransferFee: u128 = 1 * MILLIBNC;
	pub const CreationFee: u128 = 1 * MILLIBNC;
	pub const TransactionByteFee: u128 = 1 * MICROBNC;
	pub const MaxLocks: u32 = 50;
}

impl pallet_utility::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
}

impl pallet_scheduler::Config for Runtime {
	type Event = Event;
	type Origin = Origin;
	type PalletsOrigin = OriginCaller;
	type Call = Call;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const IndexDeposit: Balance = 1 * DOLLARS;
}

impl pallet_indices::Config for Runtime {
	type AccountIndex = AccountIndex;
	type Currency = Balances;
	type Deposit = IndexDeposit;
	type Event = Event;
	type WeightInfo = pallet_indices::weights::SubstrateWeight<Runtime>;
}

impl pallet_balances::Config for Runtime {
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type MaxLocks = MaxLocks;
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 5 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
	type MaxMembers = CouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const TechnicalMotionDuration: BlockNumber = 5 * DAYS;
	pub const TechnicalMaxProposals: u32 = 100;
	pub const TechnicalMaxMembers: u32 = 100;
}

type TechnicalCollective = pallet_collective::Instance2;
impl pallet_collective::Config<TechnicalCollective> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = TechnicalMotionDuration;
	type MaxProposals = TechnicalMaxProposals;
	type MaxMembers = TechnicalMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const LaunchPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
	pub const VotingPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
	pub const FastTrackVotingPeriod: BlockNumber = 3 * 24 * 60 * MINUTES;
	pub const InstantAllowed: bool = true;
	pub const MinimumDeposit: Balance = 100 * DOLLARS;
	pub const EnactmentPeriod: BlockNumber = 30 * 24 * 60 * MINUTES;
	pub const CooloffPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
	// One cent: $10,000 / MB
	pub const PreimageByteDeposit: Balance = 1 * CENTS;
	pub const MaxVotes: u32 = 100;
	pub const MaxProposals: u32 = 100;
}

impl pallet_democracy::Config for Runtime {
	type Proposal = Call;
	type Event = Event;
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type MinimumDeposit = MinimumDeposit;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin = pallet_collective::EnsureProportionAtLeast<_1, _2, AccountId, CouncilCollective>;
	/// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin = pallet_collective::EnsureProportionAtLeast<_3, _4, AccountId, CouncilCollective>;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin = pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, CouncilCollective>;
	/// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin = pallet_collective::EnsureProportionAtLeast<_2, _3, AccountId, TechnicalCollective>;
	type InstantOrigin = pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, TechnicalCollective>;
	type InstantAllowed = InstantAllowed;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to it.
	type CancellationOrigin = pallet_collective::EnsureProportionAtLeast<_2, _3, AccountId, CouncilCollective>;
	// To cancel a proposal before it has been passed, the technical committee must be unanimous or
	// Root must agree.
	type CancelProposalOrigin = EnsureOneOf<
		AccountId,
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, TechnicalCollective>,
	>;
	type BlacklistOrigin = EnsureRoot<AccountId>;
	// Any single technical committee member may veto a coming council proposal, however they can
	// only do it once and it lasts only for the cool-off period.
	type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
	type CooloffPeriod = CooloffPeriod;
	type PreimageByteDeposit = PreimageByteDeposit;
	type OperationalPreimageOrigin = pallet_collective::EnsureMember<AccountId, CouncilCollective>;
	// NOTE: Treasury replaced by `()`.
	type Slash = ();
	type Scheduler = Scheduler;
	type PalletsOrigin = OriginCaller;
	type MaxVotes = MaxVotes;
	type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
	type MaxProposals = MaxProposals;
}

impl pallet_transaction_payment::Config for Runtime {
	type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, ()>;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();
}

impl pallet_sudo::Config for Runtime {
	type Call = Call;
	type Event = Event;
}

// culumus runtime start
parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type Event = Event;
	type OnValidationData = ();
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type OutboundXcmpMessageSource = XcmpQueue;
	type DmpMessageHandler = DmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
	pub const RocLocation: MultiLocation = X1(Parent);
	pub const RococoNetwork: NetworkId = NetworkId::Polkadot;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = X1(Parachain(ParachainInfo::parachain_id().into()));
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the default `AccountId`.
	ParentIsDefault<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<RococoNetwork, AccountId>,
);

/// Means for transacting assets on this chain.
pub type LocalAssetTransactor = CurrencyAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<RocLocation>,
	// Do a simple punn to convert an AccountId32 MultiLocation into a native chain account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports.
	(),
>;

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, Origin>,
	// Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
	// recognised.
	RelayChainAsNative<RelayChainOrigin, Origin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognised.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
	// Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
	// transaction from the Root origin.
	ParentAsSuperuser<Origin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `Origin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RococoNetwork, Origin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<Origin>,
);

parameter_types! {
	// One XCM operation is 1_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: Weight = 1_000_000;
	// One ROC buys 1 second of weight.
	pub const WeightPrice: (MultiLocation, u128) = (X1(Parent), BNCS);
}

match_type! {
	pub type ParentOrParentsUnitPlurality: impl Contains<MultiLocation> = {
		X1(Parent) | X2(Parent, Plurality { id: BodyId::Unit, .. })
	};
}

pub type Barrier = (
	TakeWeightCredit,
	AllowTopLevelPaidExecutionFrom<All<MultiLocation>>,
	AllowUnpaidExecutionFrom<ParentOrParentsUnitPlurality>,
	// ^^^ Parent & its unit plurality gets free execution
);

pub struct XcmConfig;
impl Config for XcmConfig {
	type Call = Call;
	type XcmSender = XcmRouter;
	// How to withdraw and deposit an asset.
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = NativeAsset;
	type IsTeleporter = NativeAsset;	// <- should be enough to allow teleportation of ROC
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
	type Trader = UsingComponents<IdentityFee<Balance>, RocLocation, AccountId, Balances, ()>;
	type ResponseHandler = ();	// Don't handle responses for now.
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = (
	SignedToAccountId32<Origin, AccountId, RococoNetwork>,
);

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

impl pallet_xcm::Config for Runtime {
	type Event = Event;
	type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmExecuteFilter = All<(MultiLocation, Xcm<Call>)>;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = All<(MultiLocation, Vec<MultiAsset>)>;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ChannelInfo = ParachainSystem;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
}
// culumus runtime end


// bifrost runtime start

impl brml_voucher::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type WeightInfo = weights::pallet_voucher::WeightInfo<Runtime>;
}

parameter_types! {
	// 3 hours(1800 blocks) as an era
	pub const VtokenMintDuration: BlockNumber = 3 * 60 * MINUTES;
	pub const StakingPalletId: PalletId = PalletId(*b"staking ");
}
impl brml_vtoken_mint::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Assets;
	type PalletId = StakingPalletId;
	type MinterReward = MinterReward;
	// type DEXOperations = ZenlinkProtocol;
	type RandomnessSource = RandomnessCollectiveFlip;
	type WeightInfo = weights::pallet_vtoken_mint::WeightInfo<Runtime>;
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		match currency_id {
			&CurrencyId::Native(TokenSymbol::ASG) => 1 * CENTS,
			_ => Zero::zero(),
		}
	};
}

impl brml_assets::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Assets;
	type WeightInfo = ();
}

impl orml_tokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
	type MaxLocks = MaxLocks;
}

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::ASG);
}

impl brml_charge_transaction_fee::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type WeightInfo = ();
	type CurrenciesHandler = Currencies;
	type Currency = Balances;
	type OnUnbalanced = ();
	type NativeCurrencyId = NativeCurrencyId;
}

parameter_types! {
	pub const TwoYear: BlockNumber = DAYS * 365 * 2;
	pub const RewardPeriod: BlockNumber = 50;
	pub const MaximumExtendedPeriod: BlockNumber = 100;
	pub const ShareWeightPalletId: PalletId = PalletId(*b"weight  ");
}

impl brml_minter_reward::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Assets;
	type TwoYear = TwoYear;
	type SystemPalletId = ShareWeightPalletId;
	type RewardPeriod = RewardPeriod;
	type MaximumExtendedPeriod = MaximumExtendedPeriod;
	// type DEXOperations = ZenlinkProtocol;
	type ShareWeight = Balance;
}

// bifrost runtime end

// zenlink runtime start
parameter_types! {
    pub const ZenlinkPalletId: PalletId = PalletId(*b"/zenlink");
    pub const GetExchangeFee: (u32, u32) = (3, 1000);   // 0.3%

    // xcm
    pub const AnyNetwork: NetworkId = NetworkId::Any;
    pub ZenlinkRegistedParaChains: Vec<(MultiLocation, u128)> = vec![
        // Bifrost local and live, 0.01 BNC
        (make_x2_location(2001), 10_000_000_000),
        // Phala local and live, 1 PHA
        (make_x2_location(2004), 1_000_000_000_000),
        // Plasm local and live, 0.0000000000001 SDN
        (make_x2_location(2007), 1_000_000),
        // Sherpax live, 0 KSX
        (make_x2_location(2013), 0),

        // Zenlink local 1 for test
        (make_x2_location(200), 1_000_000),
        // Zenlink local 2 for test
        (make_x2_location(300), 1_000_000),
    ];
}

impl zenlink_protocol::Config for Runtime {
    type Event = Event;
    type GetExchangeFee = GetExchangeFee;
    type MultiAssetsHandler = MultiAssets;
    type PalletId = ZenlinkPalletId;
    type SelfParaId = ParachainInfo;

    type TargetChains = ZenlinkRegistedParaChains;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type Conversion = ZenlinkLocationToAccountId;
}

type MultiAssets = ZenlinkMultiAssets<ZenlinkProtocol, Balances, LocalAssetAdaptor<Currencies>>;

pub type ZenlinkLocationToAccountId = (
    // Sibling parachain origins convert to AccountId via the `ParaId::into`.
    SiblingParachainConvertsVia<Sibling, AccountId>,
    // Straight up local `AccountId32` origins just alias directly to `AccountId`.
    AccountId32Aliases<AnyNetwork, AccountId>,
);

// Below is the implementation of tokens manipulation functions other than native token.
pub struct LocalAssetAdaptor<Local>(PhantomData<Local>);

impl<Local, AccountId> LocalAssetHandler<AccountId> for LocalAssetAdaptor<Local>
where
	Local: MultiCurrency<AccountId, CurrencyId=CurrencyId>,
{
	fn local_balance_of(asset_id: AssetId, who: &AccountId) -> AssetBalance {
		match asset_id.try_into() {
			Ok(currency_id) => 
				Local::free_balance(currency_id, &who).saturated_into(),
			Err(_) => 0
		}
	}

	fn local_total_supply(asset_id: AssetId) -> AssetBalance {
		match asset_id.try_into() {
			Ok(currency_id) => Local::total_issuance(currency_id).saturated_into(),
			Err(_) => Zero::zero()
		}
	}

	fn local_is_exists(asset_id: AssetId) -> bool {
		let rs: Result<CurrencyId, ()> = asset_id.try_into();
		if rs.is_ok() {
			true
		} else {
			false
		}
	}

	fn local_transfer(
		asset_id: AssetId,
		origin: &AccountId,
		target: &AccountId,
		amount: AssetBalance,
	) -> DispatchResult {
		let currency_id: CurrencyId = asset_id.try_into().map_err(|_| DispatchError::Other("conversion from AssetId to CurrencyId failed."))?;
		Local::transfer(
			currency_id,
			&origin,
			&target,
			amount.unique_saturated_into(),
		)?;

		Ok(())
	}

	fn local_deposit(
		asset_id: AssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		let currency_id: CurrencyId = asset_id.try_into().map_err(|_| DispatchError::Other("conversion from AssetId to CurrencyId failed."))?;
		Local::deposit(currency_id, &origin, amount.unique_saturated_into())?;
		return Ok(amount)
	}

	fn local_withdraw(
		asset_id: AssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		let currency_id: CurrencyId = asset_id.try_into().map_err(|_| DispatchError::Other("conversion from AssetId to CurrencyId failed."))?;
		Local::withdraw(currency_id, &origin, amount.unique_saturated_into())?;

		Ok(amount)
	}
}


// zenlink runtime end

// orml runtime start
parameter_types! {
	pub const GetBifrostTokenId: CurrencyId = CurrencyId::Native(TokenSymbol::ASG);
}

pub type BifrostToken = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;

impl orml_currencies::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Assets;
	type NativeCurrency = BifrostToken;
	type GetNativeCurrencyId = GetBifrostTokenId;
	type WeightInfo = ();
}

// orml runtime end

construct_runtime! {
	pub enum Runtime where
		Block = Block,
		NodeBlock = generic::Block<Header, sp_runtime::OpaqueExtrinsic>,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		// Basic stuff
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 1,
		Indices: pallet_indices::{Pallet, Call, Storage, Config<T>, Event<T>} = 2,
		Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>} = 3,
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Call, Storage} = 4,
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 5,
		Democracy: pallet_democracy::{Pallet, Call, Storage, Config, Event<T>} = 6,
		Council: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 7,
		TechnicalCommittee: pallet_collective::<Instance2>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 8,

		// Parachain modules
		ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Storage, Inherent, Event<T>, ValidateUnsigned} = 12,
		ParachainInfo: parachain_info::{Pallet, Storage, Config} = 21,

		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 30,
		Utility: pallet_utility::{Pallet, Call, Event} = 41,
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 42,

		Aura: pallet_aura::{Pallet, Config<T>},
		AuraExt: cumulus_pallet_aura_ext::{Pallet, Config},

		// XCM helpers.
		XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>} = 50,
		PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin} = 51,
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Call, Event<T>, Origin} = 52,
		DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>} = 53,

		// parachain modules
		// ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Storage, Inherent, Event} = 6,
		// TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 7,
		// ParachainInfo: parachain_info::{Pallet, Storage, Config} = 8,
		// XcmHandler: cumulus_pallet_xcm_handler::{Pallet, Call, Event<T>, Origin} = 9,

		// bifrost modules
		BrmlAssets: brml_assets::{Pallet, Call, Event<T>} = 10,
		VtokenMint: brml_vtoken_mint::{Pallet, Call, Storage, Event<T>, Config<T>} = 11,
		MinterReward: brml_minter_reward::{Pallet, Storage, Event<T>, Config<T>} = 13,
		Voucher: brml_voucher::{Pallet, Call, Storage, Event<T>, Config<T>} = 14,
		ChargeTransactionFee: brml_charge_transaction_fee::{Pallet, Call, Storage, Event<T>} = 20,

		// ORML
		// XTokens: orml_xtokens::{Pallet, Storage, Call, Event<T>} = 16,
		Assets: orml_tokens::{Pallet, Storage, Event<T>, Config<T>} = 17,
		Currencies: orml_currencies::{Pallet, Call, Event<T>} = 18,

		// zenlink
		ZenlinkProtocol: zenlink_protocol::{Pallet, Call, Storage, Event<T>} = 19,
	}
}

/// The type for looking up accounts. We don't expect more than 4 billion of them.
pub type AccountIndex = u32;
/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = sp_runtime::MultiSignature;
/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as sp_runtime::traits::Verify>::Signer as sp_runtime::traits::IdentifyAccount>::AccountId;
/// Balance of an account.
pub type Balance = u128;
/// Index of a transaction in the chain.
pub type Index = u32;
/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;
/// An index to a block.
pub type BlockNumber = u32;
/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, AccountIndex>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPallets,
>;

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			Runtime::metadata().into()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(
			extrinsic: <Block as BlockT>::Extrinsic,
		) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(block: Block, data: sp_inherents::InherentData) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}

		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities()
		}
	}

	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info() -> cumulus_primitives_core::CollationInfo {
			ParachainSystem::collect_collation_info()
		}
	}

	impl brml_charge_transaction_fee_rpc_runtime_api::ChargeTransactionFeeRuntimeApi<Block, AccountId> for Runtime {
		fn get_fee_token_and_amount(who: AccountId, fee: Balance) -> (CurrencyId, Balance) {
		let rs = ChargeTransactionFee::cal_fee_token_and_amount(&who, fee);
			match rs {
				Ok(val) => val,
				_ => (CurrencyId::Native(TokenSymbol::ASG), Zero::zero()),
			}
		}
	}

	// zenlink runtime outer apis
    impl zenlink_protocol_runtime_api::ZenlinkProtocolApi<Block, AccountId> for Runtime {
        fn get_assets() -> Vec<AssetId> {
            ZenlinkProtocol::get_assets()
        }

        fn get_balance(
            asset_id: AssetId,
            owner: AccountId
        ) -> AssetBalance {
            <Runtime as zenlink_protocol::Config>::MultiAssetsHandler::balance_of(asset_id, &owner)
        }

        fn get_sovereigns_info(
            asset_id: AssetId
        ) -> Vec<(u32, AccountId, AssetBalance)> {
            ZenlinkProtocol::get_sovereigns_info(&asset_id)
        }

        fn get_all_pairs() -> Vec<PairInfo<AccountId, AssetBalance>> {
            ZenlinkProtocol::get_all_pairs()
        }

        fn get_owner_pairs(
            owner: AccountId
        ) -> Vec<PairInfo<AccountId, AssetBalance>> {
            ZenlinkProtocol::get_owner_pairs(&owner)
        }

        fn get_pair_by_asset_id(
            asset_0: AssetId,
            asset_1: AssetId
        ) -> Option<PairInfo<AccountId, AssetBalance>> {
            ZenlinkProtocol::get_pair_by_asset_id(asset_0, asset_1)
        }

        fn get_amount_in_price(
            supply: AssetBalance,
            path: Vec<AssetId>
        ) -> AssetBalance {
            ZenlinkProtocol::desired_in_amount(supply, path)
        }

        fn get_amount_out_price(
            supply: AssetBalance,
            path: Vec<AssetId>
        ) -> AssetBalance {
            ZenlinkProtocol::supply_out_amount(supply, path)
        }

        fn get_estimate_lptoken(
            token_0: AssetId,
            token_1: AssetId,
            amount_0_desired: AssetBalance,
            amount_1_desired: AssetBalance,
            amount_0_min: AssetBalance,
            amount_1_min: AssetBalance,
        ) -> AssetBalance{
            ZenlinkProtocol::get_estimate_lptoken(
                token_0,
                token_1,
                amount_0_desired,
                amount_1_desired,
                amount_0_min,
                amount_1_min
            )
        }
    }
}

cumulus_pallet_parachain_system::register_validate_block!(
	Runtime,
	cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
);