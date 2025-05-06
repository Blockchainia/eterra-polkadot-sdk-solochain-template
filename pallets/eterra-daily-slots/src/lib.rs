// pallets/eterra-daily-slots/src/lib.rs

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, traits::UnixTime};
use frame_system::pallet_prelude::*;
use sp_runtime::traits::Hash;
use sp_std::vec;
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
        #[pallet::constant]
        type MaxSlotLength: Get<u32>;
        /// How many symbols per reel
        #[pallet::constant]
        type MaxOptionsPerSlot: Get<u32>;
        /// Max rolls allowed per block
        #[pallet::constant]
        type MaxRollsPerRound: Get<u32>;
        /// Maximum number of roll results stored per account
        #[pallet::constant]
        type MaxRollHistoryLength: Get<u32>;
        #[pallet::constant]
        type MaxWeightEntries: Get<u32>;
    }

    // ─── STORAGE ────────────────────────────────────────────────────────────────

    #[derive(Encode, Decode, TypeInfo, Clone, PartialEq, Eq, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct RollResult<T: Config> {
        pub timestamp: u64,
        pub result: BoundedVec<u32, T::MaxSlotLength>,
    }

    /// (YYYY-day, count_so_far)
    #[pallet::storage]
    #[pallet::getter(fn rolls_today)]
    pub type RollsToday<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, (u64, u32), ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn last_roll_time)]
    pub type LastRollTime<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn rolls_this_block)]
    pub type RollsThisBlock<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        Blake2_128Concat,
        T::AccountId,
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

    #[pallet::storage]
    #[pallet::getter(fn roll_history)]
    pub type RollHistory<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<RollResult<T>, T::MaxRollHistoryLength>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn reel_weights)]
    pub type ReelWeights<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,                                         // reel index
        BoundedVec<(u32, u32), T::MaxWeightEntries>, // (symbol, weight)
        OptionQuery,
    >;

    // ─── EVENTS & ERRORS ───────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        SlotRolled {
            player: T::AccountId,
            result: Vec<u32>,
        },
        WeeklyWinner {
            winner: T::AccountId,
        },
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

            let slot_len = T::MaxSlotLength::get();
            let options = T::MaxOptionsPerSlot::get();
            let max_rolls = T::MaxRollsPerRound::get();
            ensure!(
                slot_len > 0 && options > 0 && max_rolls > 0,
                Error::<T>::InvalidConfiguration
            );

            // ─── DAILY CAP ──────────────────────
            let now_secs = T::TimeProvider::now().as_secs();
            let day = now_secs / 86_400; // integer day since epoch
            let (stored_day, used) = Self::rolls_today(&who);
            let used = if stored_day == day { used } else { 0 };
            ensure!(used < max_rolls, Error::<T>::ExceedRollsPerRound);

            // ─── DO THE SLOTS ───────────────────
            let mut result = Vec::with_capacity(slot_len as usize);
            for i in 0..slot_len {
                // Fetch weights from storage for this reel
                let weights = ReelWeights::<T>::get(i).ok_or(Error::<T>::InvalidConfiguration)?;
                log::info!("Using weights for reel {}: {:?}", i, weights);

                // Create unique input per reel
                let entropy = (now_secs, &who, i, frame_system::Pallet::<T>::block_number());
                let hash = T::Hashing::hash_of(&entropy);

                // Weighted selection logic
                let total_weight = weights.iter().map(|(_, w)| *w).sum::<u32>();
                let rand_seed = u32::from_le_bytes([
                    hash.as_ref()[0],
                    hash.as_ref()[1],
                    hash.as_ref()[2],
                    hash.as_ref()[3],
                ]);
                let rand_index = rand_seed % total_weight;

                let mut acc = 0;
                let chosen_symbol = weights
                    .iter()
                    .find_map(|(symbol, weight)| {
                        acc += *weight;
                        if rand_index < acc {
                            Some(*symbol)
                        } else {
                            None
                        }
                    })
                    .ok_or(Error::<T>::InvalidConfiguration)?;

                result.push(chosen_symbol);
            }

            // ─── UPDATE STATE ───────────────────
            // bump that user’s count for *this* day
            RollsToday::<T>::insert(&who, (day, used + 1));
            LastRollTime::<T>::insert(&who, now_secs);

            // ─── AWARD TICKETS ──────────────────
            let ticket_symbol = 7u32;
            let tickets = result.iter().filter(|&&v| v == ticket_symbol).count() as u32;
            if tickets > 0 {
                TicketsPerUser::<T>::mutate(&who, |t| *t += tickets);
                TotalTickets::<T>::mutate(|t| *t += tickets);
            }

            Self::deposit_event(Event::SlotRolled {
                player: who.clone(),
                result: result.clone(),
            });

            // Save the roll result
            let bounded_result: BoundedVec<_, T::MaxSlotLength> = result
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::InvalidConfiguration)?;

            let roll_entry = RollResult::<T> {
                timestamp: now_secs,
                result: bounded_result,
            };

            RollHistory::<T>::mutate(&who, |history| {
                if history.len() as u32 >= T::MaxRollHistoryLength::get() {
                    history.remove(0);
                }
                let _ = history.try_push(roll_entry);
            });

            Ok(())
        }

        /// This sets the weights for one reel (indexed by `reel`).
        /// To bias results, ensure all reels (from 0 to MaxSlotLength - 1) are updated.
        #[pallet::call_index(1)]
        #[pallet::weight(10_000)]
        pub fn set_reel_weights(
            origin: OriginFor<T>,
            reel: u32,
            weights: Vec<(u32, u32)>,
        ) -> DispatchResult {
            ensure_root(origin)?; // or ensure_signed(origin)? with checks

            let bounded: BoundedVec<_, T::MaxWeightEntries> = weights
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::InvalidConfiguration)?;

            ReelWeights::<T>::insert(reel, bounded);
            log::info!(
                "Set weights for reel {}: {:?}",
                reel,
                weights
            );
            Ok(())
        }

        /// This allows a root origin to update multiple reels' weights in one call.
        #[pallet::call_index(2)]
        #[pallet::weight(10_000)]
        pub fn set_all_reel_weights(
            origin: OriginFor<T>,
            all_weights: Vec<(u32, Vec<(u32, u32)>)>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            for (reel, weights) in all_weights {
                let bounded: BoundedVec<_, T::MaxWeightEntries> = weights
                    .clone()
                    .try_into()
                    .map_err(|_| Error::<T>::InvalidConfiguration)?;

                ReelWeights::<T>::insert(reel, bounded);
                log::info!("Set weights for reel {}: {:?}", reel, weights);
            }

            Ok(())
        }
    }

    // ─── INTERNAL ───────────────────────────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        fn perform_weekly_drawing() -> Result<(), Error<T>> {
            let total = TotalTickets::<T>::get();
            if total == 0 {
                return Err(Error::<T>::NoTicketsAvailable);
            }
            let now = T::TimeProvider::now().as_secs();
            let seed = T::Hashing::hash_of(&(now, frame_system::Pallet::<T>::block_number()));
            let pick = (seed.as_ref()[0] as u32) % total;

            let mut cum = 0;
            for (acct, share) in TicketsPerUser::<T>::iter() {
                cum += share;
                if pick < cum {
                    Self::deposit_event(Event::WeeklyWinner {
                        winner: acct.clone(),
                    });
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
            // Only on the first block
            if _n == 1u32.into() {
                let default_weights = vec![
                    (0, vec![(0, 5), (1, 3), (2, 2)]),
                    (1, vec![(0, 1), (1, 1), (2, 8)]),
                    (2, vec![(0, 4), (1, 4), (2, 2)]),
                ];

                for (reel, weights) in default_weights {
                    if !ReelWeights::<T>::contains_key(reel) {
                        let bounded: BoundedVec<_, T::MaxWeightEntries> =
                            weights.try_into().expect("Hardcoded weights are valid");
                        ReelWeights::<T>::insert(reel, bounded);
                    }
                }
            }

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
