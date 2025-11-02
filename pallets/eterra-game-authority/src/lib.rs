#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        BoundedBTreeSet,
        BoundedVec,
    };
    use frame_system::pallet_prelude::*;
    use frame_system::pallet_prelude::BlockNumberFor;
    use sp_std::marker::PhantomData;
    use sp_std::vec::Vec;
    use frame_support::traits::BuildGenesisConfig;
    use frame_support::sp_runtime::traits::Saturating;

    pub type GameId = u64;

    #[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(MaxPlayers))]
    pub struct GameInfo<AccountId, MaxPlayers: Get<u32>> {
        pub server: AccountId,
        pub players: BoundedBTreeSet<AccountId, MaxPlayers>,
        pub started: bool,
        pub ended: bool,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        #[pallet::constant]
        type MaxPlayersPerGame: Get<u32>;
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Maximum number of expirations processed in a single block (bounds on_initialize work)
        #[pallet::constant]
        type MaxExpirationsPerBlock: Get<u32>;

        /// Maximum round length, in blocks. Games exceeding this age are auto-ended.
        type MaxRoundBlocks: Get<BlockNumberFor<Self>>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn next_game_id)]
    pub type NextGameId<T: Config> = StorageValue<_, GameId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn games)]
    pub type Games<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        GameId,
        GameInfo<T::AccountId, T::MaxPlayersPerGame>,
        OptionQuery
    >;

    #[pallet::storage]
    #[pallet::getter(fn eliminations)]
    pub type Eliminations<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat, GameId,
        Blake2_128Concat, T::AccountId,
        u32,
        ValueQuery
    >;

    #[pallet::storage]
    #[pallet::getter(fn is_server_whitelisted)]
    pub type WhitelistedServers<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, (), OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn active_game_by_player)]
    pub type ActiveGameByPlayer<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        GameId,
        OptionQuery
    >;

    /// BlockNumber => list of game IDs scheduled to auto-end at that block.
    #[pallet::storage]
    #[pallet::getter(fn expirations)]
    pub type Expirations<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<GameId, T::MaxExpirationsPerBlock>,
        ValueQuery
    >;
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            // Take the list of games scheduled to expire now.
            let mut weight: Weight = T::DbWeight::get().reads_writes(1, 0);
            let games: BoundedVec<GameId, T::MaxExpirationsPerBlock> = Expirations::<T>::take(n);
            // For each game, end it if not already ended.
            for game_id in games.into_inner().into_iter() {
                if let Some(mut game) = Games::<T>::get(game_id) {
                    weight = weight.saturating_add(T::DbWeight::get().reads_writes(2, 2));
                    if !game.ended {
                        // mark ended and clear active mapping for players
                        game.ended = true;
                        let players: Vec<T::AccountId> = game.players.iter().cloned().collect();
                        for p in players {
                            ActiveGameByPlayer::<T>::remove(&p);
                        }
                        Games::<T>::insert(game_id, game);
                        // Emit event so indexers know this was auto-ended.
                        Self::deposit_event(Event::GameEnded(game_id));
                    }
                } else {
                    // account a read
                    weight = weight.saturating_add(T::DbWeight::get().reads(1));
                }
            }
            weight
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ServerWhitelisted(T::AccountId),
        ServerRemoved(T::AccountId),
        GameCreated(GameId, T::AccountId),
        PlayerAdded(GameId, T::AccountId),
        EliminationsRecorded(GameId, T::AccountId, u32, u32),
        GameEnded(GameId),
    }

    #[pallet::error]
    pub enum Error<T> {
        GameNotFound,
        GameAlreadyEnded,
        GameNotStarted,
        GameFull,
        PlayerAlreadyInGame,
        PlayerNotInGame,
        PlayerInAnotherActiveGame,
        NotWhitelistedServer,
        AlreadyWhitelisted,
        NotWhitelisted,
        NotGameOwnerServer,
    }

    impl<T: Config> Pallet<T> {
        fn ensure_whitelisted(who: &T::AccountId) -> Result<(), Error<T>> {
            WhitelistedServers::<T>::contains_key(who).then_some(()).ok_or(Error::<T>::NotWhitelistedServer)
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn add_server(origin: T::RuntimeOrigin, server: T::AccountId) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            ensure!(!WhitelistedServers::<T>::contains_key(&server), Error::<T>::AlreadyWhitelisted);
            WhitelistedServers::<T>::insert(&server, ());
            Self::deposit_event(Event::ServerWhitelisted(server));
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn remove_server(origin: T::RuntimeOrigin, server: T::AccountId) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            ensure!(WhitelistedServers::<T>::contains_key(&server), Error::<T>::NotWhitelisted);
            WhitelistedServers::<T>::remove(&server);
            Self::deposit_event(Event::ServerRemoved(server));
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn create_game(origin: T::RuntimeOrigin) -> DispatchResult {
            let server = ensure_signed(origin)?;
            Self::ensure_whitelisted(&server)?;
            let id = NextGameId::<T>::get();
            let info = GameInfo::<T::AccountId, T::MaxPlayersPerGame> {
                server: server.clone(),
                players: BoundedBTreeSet::new(),
                started: true,
                ended: false,
            };
            Games::<T>::insert(id, info);
            NextGameId::<T>::put(id.saturating_add(1));

            // Schedule automatic end of game after MaxRoundBlocks from now.
            let now = <frame_system::Pallet<T>>::block_number();
            let expire_at = now.saturating_add(T::MaxRoundBlocks::get());
            Expirations::<T>::mutate(expire_at, |list| {
                let _ = list.try_push(id);
            });

            Self::deposit_event(Event::GameCreated(id, server));
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn add_player(origin: T::RuntimeOrigin, game_id: GameId, player: T::AccountId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Self::ensure_whitelisted(&caller)?;
            Games::<T>::try_mutate(game_id, |maybe_game| -> DispatchResult {
                let game = maybe_game.as_mut().ok_or(Error::<T>::GameNotFound)?;
                ensure!(caller == game.server, Error::<T>::NotGameOwnerServer);
                ensure!(game.started, Error::<T>::GameNotStarted);
                ensure!(!game.ended, Error::<T>::GameAlreadyEnded);
                ensure!(!game.players.contains(&player), Error::<T>::PlayerAlreadyInGame);
                ensure!(ActiveGameByPlayer::<T>::get(&player).is_none(), Error::<T>::PlayerInAnotherActiveGame);
                game.players.try_insert(player.clone()).map_err(|_| Error::<T>::GameFull)?;
                ActiveGameByPlayer::<T>::insert(&player, game_id);
                Ok(())
            })?;
            Self::deposit_event(Event::PlayerAdded(game_id, player));
            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn record_eliminations(origin: T::RuntimeOrigin, game_id: GameId, player: T::AccountId, delta: u32) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Self::ensure_whitelisted(&caller)?;
            Games::<T>::try_mutate_exists(game_id, |maybe_game| -> DispatchResult {
                let game = maybe_game.as_mut().ok_or(Error::<T>::GameNotFound)?;
                ensure!(caller == game.server, Error::<T>::NotGameOwnerServer);
                ensure!(game.started, Error::<T>::GameNotStarted);
                ensure!(!game.ended, Error::<T>::GameAlreadyEnded);
                ensure!(game.players.contains(&player), Error::<T>::PlayerNotInGame);
                let new_total = Eliminations::<T>::mutate(game_id, &player, |count| {
                    *count = count.saturating_add(delta);
                    *count
                });
                Self::deposit_event(Event::EliminationsRecorded(game_id, player.clone(), delta, new_total));
                Ok(())
            })
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn end_game(origin: T::RuntimeOrigin, game_id: GameId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Self::ensure_whitelisted(&caller)?;
            Games::<T>::try_mutate(game_id, |maybe_game| -> DispatchResult {
                let game = maybe_game.as_mut().ok_or(Error::<T>::GameNotFound)?;
                ensure!(caller == game.server, Error::<T>::NotGameOwnerServer);
                ensure!(!game.ended, Error::<T>::GameAlreadyEnded);
                game.ended = true;
                let players: Vec<T::AccountId> = game.players.iter().cloned().collect();
                for p in players {
                    ActiveGameByPlayer::<T>::remove(&p);
                }
                Ok(())
            })?;
            Self::deposit_event(Event::GameEnded(game_id));
            Ok(())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub initial_servers: Vec<T::AccountId>,
        pub _phantom: PhantomData<T>,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self { initial_servers: Vec::new(), _phantom: Default::default() }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            for server in &self.initial_servers {
                WhitelistedServers::<T>::insert(server, ());
            }
        }
    }
}

// --- Test modules ---
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
