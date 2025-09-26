#![cfg_attr(not(feature = "std"), no_std)]

pub mod eterra_adapter {
    use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
    use scale_info::TypeInfo;

    // Local copies of game types so this adapter has no dependency on pallet-eterra
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug)]
    pub enum Possession { PlayerOne, PlayerTwo }

    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug, Default)]
    pub struct Card {
        pub top: u8,
        pub right: u8,
        pub bottom: u8,
        pub left: u8,
        pub possession: Option<Possession>,
    }

    pub type Board = [[Option<Card>; 4]; 4];

    /// One hand entry (mirrors data needed to place a card)
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug, Default)]
    pub struct HandEntry {
        pub north: u8,
        pub east: u8,
        pub south: u8,
        pub west: u8,
        pub used: bool,
    }

    /// Fixed-size hand (5 entries). If you make HandSize configurable later,
    /// you can rework this to a BoundedVec, but fixed-size is fastest for AI.
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug, Default)]
    pub struct Hand {
        pub entries: [HandEntry; 5],
    }

    /// Compact, cloneable snapshot of game state used by the AI
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug)]
    pub struct State {
        pub board: Board,   // 4x4 Option<Card>
        pub scores: (u8, u8),     // (p0, p1)
        pub player_turn: u8,      // 0 or 1
        pub round: u8,
        pub max_rounds: u8,
        pub hands: [Hand; 2],
    }

    impl Default for State {
        fn default() -> Self {
            Self {
                board: Default::default(),
                scores: (0, 0),
                player_turn: 0,
                round: 0,
                max_rounds: 0,
                hands: [Hand::default(), Hand::default()],
            }
        }
    }

    /// Play a card from hand at (x,y)
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Debug, Default)]
    pub struct Action {
        pub hand_index: u8, // 0..4
        pub x: u8,          // 0..3
        pub y: u8,          // 0..3
    }

    /// Adapter gluing your card game rules to the generic Monte-Carlo AI.
    pub struct Adapter;

    impl Default for Adapter {
        fn default() -> Self { Adapter }
    }

    impl Adapter {
        /// Pure helper: list actions without relying on trait resolution.
        pub fn list_actions_pure<const MAX: usize>(
            s: &State,
            out: &mut [Option<Action>; MAX],
        ) -> usize {
            if s.round >= s.max_rounds {
                return 0;
            }
            let mut k = 0;
            for x in 0..4u8 {
                for y in 0..4u8 {
                    if s.board[x as usize][y as usize].is_some() {
                        continue;
                    }
                    for (idx, he) in s.hands[s.player_turn as usize].entries.iter().enumerate() {
                        if he.used {
                            continue;
                        }
                        if k < MAX {
                            out[k] = Some(Action { hand_index: idx as u8, x, y });
                            k += 1;
                            if k == MAX {
                                return k;
                            }
                        }
                    }
                }
            }
            k
        }

        /// Pure helper: apply action without relying on trait resolution.
        pub fn apply_pure(s: &State, a: &Action) -> State {
            let mut g = s.clone();

            // Build a placed card from hand entry
            let he = g.hands[g.player_turn as usize].entries[a.hand_index as usize].clone();
            let mut placed = Card {
                top: he.north,
                right: he.east,
                bottom: he.south,
                left: he.west,
                possession: None,
            };

            let placing_player = if g.player_turn == 0 { Possession::PlayerOne } else { Possession::PlayerTwo };
            placed.possession = Some(placing_player.clone());

            // Place on board
            g.board[a.x as usize][a.y as usize] = Some(placed.clone());

            // Capture logic (mirrors pallet)
            let dirs = [
                (0i8, -1i8, placed.top),
                (1, 0, placed.right),
                (0, 1, placed.bottom),
                (-1, 0, placed.left),
            ];

            for &(dx, dy, opposing_rank) in &dirs {
                let nx = a.x as i8 + dx;
                let ny = a.y as i8 + dy;
                if nx >= 0 && nx < 4 && ny >= 0 && ny < 4 {
                    if let Some(mut opp) = g.board[nx as usize][ny as usize].clone() {
                        let rank = match (dx, dy) {
                            (0, -1) => opp.bottom,
                            (1, 0) => opp.left,
                            (0, 1) => opp.top,
                            (-1, 0) => opp.right,
                            _ => 0,
                        };
                        if opposing_rank > rank {
                            if let Some(prev) = opp.possession.clone() {
                                if prev == Possession::PlayerOne {
                                    g.scores.0 = g.scores.0.saturating_sub(1);
                                } else if prev == Possession::PlayerTwo {
                                    g.scores.1 = g.scores.1.saturating_sub(1);
                                }
                            }
                            if placing_player == Possession::PlayerOne {
                                g.scores.0 = g.scores.0.saturating_add(1);
                            } else {
                                g.scores.1 = g.scores.1.saturating_add(1);
                            }
                            opp.possession = Some(placing_player.clone());
                            g.board[nx as usize][ny as usize] = Some(opp);
                        }
                    }
                }
            }

            // Mark used & advance turn/round (increment round on wrap)
            g.hands[g.player_turn as usize].entries[a.hand_index as usize].used = true;
            if g.player_turn == 0 {
                g.player_turn = 1;
            } else {
                g.player_turn = 0;
                g.round = g.round.saturating_add(1);
            }
            g
        }
    }

    impl pallet_eterra_monte_carlo_ai::GameAdapter for Adapter {
        type State = State;
        type Action = Action;
        type Player = u8;

        fn list_actions<const MAX: usize>(
            s: &<Self as pallet_eterra_monte_carlo_ai::GameAdapter>::State,
            out: &mut [Option<<Self as pallet_eterra_monte_carlo_ai::GameAdapter>::Action>; MAX],
        ) -> usize {
            Adapter::list_actions_pure::<MAX>(s, out)
        }

        fn apply(
            s: &<Self as pallet_eterra_monte_carlo_ai::GameAdapter>::State,
            a: &<Self as pallet_eterra_monte_carlo_ai::GameAdapter>::Action,
        ) -> <Self as pallet_eterra_monte_carlo_ai::GameAdapter>::State {
            Adapter::apply_pure(s, a)
        }

        fn is_terminal(s: &<Self as pallet_eterra_monte_carlo_ai::GameAdapter>::State) -> bool {
            s.round >= s.max_rounds
        }

        fn current_player(s: &<Self as pallet_eterra_monte_carlo_ai::GameAdapter>::State) -> Self::Player {
            s.player_turn
        }

        fn score(
            s: &<Self as pallet_eterra_monte_carlo_ai::GameAdapter>::State,
            for_player: Self::Player,
        ) -> i32 {
            let (a, b) = s.scores;
            if for_player == 0 { (a as i32) - (b as i32) } else { (b as i32) - (a as i32) }
        }

        fn random_action(
            s: &<Self as pallet_eterra_monte_carlo_ai::GameAdapter>::State,
            seed: u64,
        ) -> Option<<Self as pallet_eterra_monte_carlo_ai::GameAdapter>::Action> {
            const MAX: usize = 128;
            let mut buf: [Option<<Self as pallet_eterra_monte_carlo_ai::GameAdapter>::Action>; MAX] =
                core::array::from_fn(|_| None);
            let n = <Self as pallet_eterra_monte_carlo_ai::GameAdapter>::list_actions::<MAX>(s, &mut buf);
            if n == 0 { return None; }
            let idx = (seed as usize) % n;
            buf[idx].clone()
        }
    }
}