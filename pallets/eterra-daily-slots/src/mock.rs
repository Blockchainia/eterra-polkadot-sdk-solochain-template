#![cfg(test)]
//! Mock runtime and helpers for pallet-eterra-daily-slots

use crate as pallet_eterra_daily_slots;
use crate::Config;
use frame_support::BoundedVec;
use frame_support::{
    construct_runtime, parameter_types,
    traits::{ConstU16, ConstU32, Everything, UnixTime},
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::traits::{BlakeTwo256, IdentityLookup};
use sp_runtime::BuildStorage;
use std::cell::Cell;
use std::time::Duration;

// =====================================================
// 🕰️ Mock Time Provider
// =====================================================
thread_local! {
    // each test thread gets its own clock, defaulting to 90_000
    static MOCK_NOW: Cell<u64> = Cell::new(90_000);
}

/// A `UnixTime` implementation that reads from our thread-local clock.
pub struct MockTime;
impl UnixTime for MockTime {
    fn now() -> Duration {
        let secs = MOCK_NOW.with(|c| c.get());
        Duration::from_secs(secs)
    }
}

/// Helpers to manipulate the thread-local clock.
pub struct MockTimeState;
impl MockTimeState {
    /// Reset to a known baseline (90 000).
    pub fn set_now(new_now: u64) {
        MOCK_NOW.with(|c| c.set(new_now));
    }
    /// Read it back (if needed).
    pub fn now() -> u64 {
        MOCK_NOW.with(|c| c.get())
    }
}

// =====================================================
// 🛠 Mock Runtime
// =====================================================
construct_runtime!(
    pub enum Test {
        System: system,
        EterraDailySlots: pallet_eterra_daily_slots,
    }
);

type UncheckedExtrinsic = system::mocking::MockUncheckedExtrinsic<Test>;
type Block = system::mocking::MockBlock<Test>;
pub type TestRuntime = Test;

// =====================================================
// ⚙ frame_system::Config for TestRuntime
// =====================================================
parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl system::Config for Test {
    // core
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeTask = (); // new
    type Lookup = IdentityLookup<Self::AccountId>;
    type AccountId = u64;
    type Nonce = u64; // missing
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Block = Block; // missing
    type BlockHashCount = BlockHashCount; // missing
    type Version = (); // missing
    type PalletInfo = PalletInfo; // missing

    // balances-like
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();

    // weights & limits
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ConstU16<42>;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;

    // in/out hooks (new)
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
}

// =====================================================
// ⚙ pallet_eterra_daily_slots::Config for TestRuntime
// =====================================================
parameter_types! {
    pub const MaxSlotLength:     u32 = 3;
    pub const MaxOptionsPerSlot: u32 = 5;
    pub const MaxRollsPerRound:  u32 = 3;
    pub const MaxRollHistoryLength: u32 = 100;
    pub const MaxWeightEntries: u32 = 10;
}

impl pallet_eterra_daily_slots::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type TimeProvider = MockTime;
    type MaxSlotLength = MaxSlotLength;
    type MaxOptionsPerSlot = MaxOptionsPerSlot;
    type MaxRollsPerRound = MaxRollsPerRound;
    type MaxRollHistoryLength = MaxRollHistoryLength;
    type MaxWeightEntries = MaxWeightEntries;
}

// =====================================================
// 🧪 Externalities Builder
// =====================================================

fn reset_mock_time() {
    MockTimeState::set_now(90_000);
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    // build the initial storage from genesis
    let storage = system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("genesis build failed");
    let mut ext = sp_io::TestExternalities::from(storage);

    ext.execute_with(|| {
        MockTimeState::set_now(90_000);
        frame_system::Pallet::<Test>::set_block_number(1);

        // Clear storage
        let _ = crate::LastRollTime::<TestRuntime>::clear(u32::MAX, None);
        let _ = crate::RollsThisBlock::<TestRuntime>::clear(u32::MAX, None);
        let _ = crate::TicketsPerUser::<TestRuntime>::clear(u32::MAX, None);
        let _ = crate::TotalTickets::<TestRuntime>::kill();
        let _ = crate::LastDrawingTime::<TestRuntime>::kill();

        // 🆕 Set default weights for each reel to prevent panics
        for reel in 0..<Test as Config>::MaxSlotLength::get() {
            let weights = vec![(0, 1), (1, 1), (2, 1)];
            let bounded: BoundedVec<_, MaxWeightEntries> =
                weights.try_into().expect("Failed to create bounded vec");
            crate::ReelWeights::<Test>::insert(reel, bounded);
        }
    });

    ext
}
