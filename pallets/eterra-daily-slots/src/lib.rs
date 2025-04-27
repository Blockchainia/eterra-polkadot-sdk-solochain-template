// pallets/eterra-daily-slots/src/lib.rs

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, traits::UnixTime};
use frame_system::pallet_prelude::*;
use sp_runtime::traits::{Hash, SaturatedConversion};
use sp_std::vec::Vec;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// Configuration trait for this pallet.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The outer event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Time provider
        type TimeProvider: UnixTime;

        /// How many reels (slots)
        #[pallet::constant] type MaxSlotLength: Get<u32>;
        /// How many symbols per reel
        #[pallet::constant] type MaxOptionsPerSlot: Get<u32>;
        /// Max rolls allowed per block
        #[pallet::constant] type MaxRollsPerRound: Get<u32>;
    }

    // ─── STORAGE ────────────────────────────────────────────────────────────────

    #[pallet::storage]
    #[pallet::getter(fn last_roll_time)]
    pub type LastRollTime<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn rolls_this_block)]
    pub type RollsThisBlock<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat, BlockNumberFor<T>,
        Blake2_128Concat, T::AccountId,
        u32,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn tickets_per_user)]
    pub type TicketsPerUser<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn total_tickets)]
    pub type TotalTickets<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn last_drawing_time)]
    pub type LastDrawingTime<T: Config> = StorageValue<_, u64, ValueQuery>;

    // ─── EVENTS & ERRORS ───────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        SlotRolled { player: T::AccountId, result: Vec<u32> },
        WeeklyWinner { winner: T::AccountId },
    }

    #[pallet::error]
    pub enum Error<T> {
        RollNotAvailableYet,
        ExceedRollsPerRound,
        InvalidConfiguration,
        NoTicketsAvailable,
    }

    // ─── DISPATCHABLE CALLS ───────────────────────────────────────────────────

   
// pallets/eterra-daily-slots/src/lib.rs

#[pallet::call]
impl<T: Config> Pallet<T> {
    #[pallet::call_index(0)]
    #[pallet::weight(10_000)]
    pub fn roll(origin: OriginFor<T>) -> DispatchResult {
        let who = ensure_signed(origin)?;

        // ─── LOAD CONFIG FROM STORAGE ───────────────────────────────
        // 📌 Pull directly from the runtime constants:
        let slot_len       = T::MaxSlotLength::get();
        let options_per_slot  = T::MaxOptionsPerSlot::get();
        let max_rolls      = T::MaxRollsPerRound::get();        ensure!(
            slot_len > 0 && options_per_slot > 0 && max_rolls > 0,
            Error::<T>::InvalidConfiguration
        );

        // ─── ENFORCE ONE ROLL PER BLOCK ──────────────────────────────
        let current_block = frame_system::Pallet::<T>::block_number();
        let used = RollsThisBlock::<T>::get(current_block, &who);
        ensure!(used < max_rolls, Error::<T>::ExceedRollsPerRound);

        // ─── ENFORCE 24H COOLDOWN ────────────────────────────────────
        let now = T::TimeProvider::now().as_secs();
        let last_roll = LastRollTime::<T>::get(&who);
        ensure!(now >= last_roll + 86_400, Error::<T>::RollNotAvailableYet);

        // ─── DO THE SLOTS ────────────────────────────────────────────
        let mut result = Vec::with_capacity(slot_len as usize);
        for _ in 0..slot_len {
            let roll_value = (now % (options_per_slot as u64)) as u32;
            result.push(roll_value);
        }

        // ─── UPDATE STATE ───────────────────────────────────────────
        RollsThisBlock::<T>::insert(current_block, &who, used + 1);
        LastRollTime::<T>::insert(&who, now);

        // ─── AWARD TICKETS ──────────────────────────────────────────
        let ticket_symbol: u32 = 7;
        let ticket_count = result.iter().filter(|&&v| v == ticket_symbol).count() as u32;
        if ticket_count > 0 {
            TicketsPerUser::<T>::mutate(&who, |t| *t += ticket_count);
            TotalTickets::<T>::mutate(|total| *total += ticket_count);
        }

        Self::deposit_event(Event::SlotRolled { player: who, result });
        Ok(())
    }
}

    // ─── INTERNAL ───────────────────────────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        fn perform_weekly_drawing() -> Result<(), Error<T>> {
            let total = TotalTickets::<T>::get();
            if total == 0 {
                return Err(Error::<T>::NoTicketsAvailable)
            }
            let now  = T::TimeProvider::now().as_secs();
            let seed = T::Hashing::hash_of(&(now, frame_system::Pallet::<T>::block_number()));
            let pick = (seed.as_ref()[0] as u32) % total;

            let mut cum = 0;
            for (acct, share) in TicketsPerUser::<T>::iter() {
                cum += share;
                if pick < cum {
                    Self::deposit_event(Event::WeeklyWinner { winner: acct.clone() });
                    break;
                }
            }

            // reset
            let _ = TicketsPerUser::<T>::clear(u32::MAX, None);
            TotalTickets::<T>::put(0);
            LastDrawingTime::<T>::put(now);
            Ok(())
        }
    }

    // ─── HOOKS ────────────────────────────────────────────────────────────────

use frame_support::weights::Weight;

#[pallet::hooks]
impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
    fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
        // Grab “now” once:
        let now_secs = T::TimeProvider::now().as_secs();

        // How many seconds have elapsed since UNIX epoch in days:
        let days_since_epoch = now_secs / 86_400;
        // Adjust so that day_of_week == 0 means Sunday:
        let day_of_week = (days_since_epoch + 4) % 7;

        // How many seconds into *today* we are:
        let secs_today = now_secs % 86_400;

        // Only run the weekly drawing if *both*:
        //   1) it's Sunday (day_of_week == 0), and
        //   2) it's at or after 18:00 (18 * 3600 = 64800)
        let is_sunday = day_of_week == 0;
        let is_after_6pm = secs_today >= 18 * 3600;
        if !(is_sunday && is_after_6pm) {
            // bail out early, no drawing
            return Weight::from_parts(10_000, 0);
        }

        // If we’ve already done a drawing in the last 24 h, bail again:
        let last = LastDrawingTime::<T>::get();
        if now_secs.saturating_sub(last) < 24 * 3600 {
            return Weight::from_parts(10_000, 0);
        }

        // Now we really do a weekly drawing
        if let Err(e) = Self::perform_weekly_drawing() {
            log::warn!("(eterra-daily-slots) weekly drawing failed: {:?}", e);
        }

        Weight::from_parts(10_000, 0)
    }
}
}

pub use pallet::*;

/// Mock & tests live here
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;