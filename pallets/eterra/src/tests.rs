use crate::{mock::*, Card};
use frame_support::{assert_ok, assert_noop};
use sp_runtime::traits::{BlakeTwo256, Hash};
use std::sync::Once;
use log::{Record, Level, Metadata};
use sp_core::H256; // Fix: Import H256


static INIT: Once = Once::new();

pub struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!(
                "[{}] {}: {}",
                record.level(),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

static LOGGER: SimpleLogger = SimpleLogger;

pub fn init_logger() {
    INIT.call_once(|| {
        log::set_logger(&LOGGER).unwrap();
        log::set_max_level(log::LevelFilter::Debug);
    });
}

fn setup_new_game() -> (H256, u64, u64) {
    let creator = 1;
    let opponent = 2;

    let game_id = BlakeTwo256::hash_of(&(creator, opponent));
    assert_ok!(Eterra::create_game(
        frame_system::RawOrigin::Signed(creator).into(),
        opponent
    ));

    log::debug!("Game created with ID: {:?}, Creator: {}, Opponent: {}", game_id, creator, opponent);
    (game_id, creator, opponent)
}

#[test]
fn create_game_with_same_players_fails() {
    init_logger();
    new_test_ext().execute_with(|| {
        let player = 1; // Define `player` explicitly
        let result = Eterra::create_game(
            frame_system::RawOrigin::Signed(player).into(),
            player,
        );

        assert_noop!(result, crate::Error::<Test>::InvalidMove);
    });
}


#[test]
fn invalid_move_on_occupied_cell() {
    init_logger();
    new_test_ext().execute_with(|| {
        let (game_id, creator, opponent) = setup_new_game();

        let card = Card::new(5, 3, 2, 4);
        assert_ok!(Eterra::play_turn(
            frame_system::RawOrigin::Signed(creator).into(),
            game_id,
            1,
            1,
            card.clone()
        ));

        // Attempt to play on the same cell
        let result = Eterra::play_turn(
            frame_system::RawOrigin::Signed(opponent).into(),
            game_id,
            1,
            1,
            card
        );

        assert_noop!(result, crate::Error::<Test>::CellOccupied);
    });
}

#[test]
fn create_game_works() {
    init_logger();
    new_test_ext().execute_with(|| {
        let (game_id, creator, opponent) = setup_new_game();

        let (board, game_creator, game_opponent) = Eterra::game_board(game_id).unwrap();
        let current_turn = Eterra::current_turn(game_id).unwrap();

        assert_eq!(game_creator, creator);
        assert_eq!(game_opponent, opponent);
        assert!(current_turn == creator || current_turn == opponent);
        assert!(board.iter().flatten().all(|cell| cell.is_none())); // Verify empty board
    });
}

#[test]
fn play_turn_works() {
    init_logger();
    new_test_ext().execute_with(|| {
        let (game_id, creator, opponent) = setup_new_game();

        let card = Card::new(5, 3, 2, 4);
        assert_ok!(Eterra::play_turn(
            frame_system::RawOrigin::Signed(creator).into(),
            game_id,
            1,
            1,
            card.clone()
        ));

        log::debug!(
            "After creator's turn, current turn: {:?}",
            Eterra::current_turn(game_id).unwrap()
        );

        let (board, _, _) = Eterra::game_board(game_id).unwrap();
        assert_eq!(board[1][1], Some(card));

        let opponent_card = Card::new(2, 4, 5, 3);
        assert_ok!(Eterra::play_turn(
            frame_system::RawOrigin::Signed(opponent).into(),
            game_id,
            1,
            2,
            opponent_card.clone()
        ));

        log::debug!(
            "After opponent's turn, current turn: {:?}",
            Eterra::current_turn(game_id).unwrap()
        );

        let (updated_board, _, _) = Eterra::game_board(game_id).unwrap();
        assert_eq!(updated_board[1][2], Some(opponent_card));
    });
}

#[test]
fn card_capture_multiple_directions() {
    init_logger();
    new_test_ext().execute_with(|| {
        let (game_id, creator, opponent) = setup_new_game();

        // Creator's first move
        let creator_card = Card::new(3, 5, 2, 1);
        assert_ok!(Eterra::play_turn(
            frame_system::RawOrigin::Signed(creator).into(),
            game_id,
            0,
            0,
            creator_card.clone()
        ));

        // Opponent's first move
        let opponent_card = Card::new(2, 4, 5, 3);
        assert_ok!(Eterra::play_turn(
            frame_system::RawOrigin::Signed(opponent).into(),
            game_id,
            0,
            1,
            opponent_card.clone()
        ));

        // Creator's second move captures two cards
        let capturing_card = Card::new(6, 6, 3, 3);
        assert_ok!(Eterra::play_turn(
            frame_system::RawOrigin::Signed(creator).into(),
            game_id,
            0,
            2,
            capturing_card.clone()
        ));

        // Validate board state
        let (board, _, _) = Eterra::game_board(game_id).unwrap();
        assert_eq!(board[0][1], Some(capturing_card.clone())); // Capturing card
        assert_eq!(board[0][2], Some(capturing_card));         // Captured card
    });
}

#[test]
fn play_turn_out_of_bounds_fails() {
    init_logger();
    new_test_ext().execute_with(|| {
        let (game_id, creator, _) = setup_new_game();

        let card = Card::new(5, 3, 2, 4);
        let result = Eterra::play_turn(
            frame_system::RawOrigin::Signed(creator).into(),
            game_id,
            5, // Invalid X coordinate
            1, // Valid Y coordinate
            card,
        );

        assert_noop!(result, crate::Error::<Test>::InvalidMove);
    });
}

#[test]
fn full_game_simulation() {
    init_logger();
    new_test_ext().execute_with(|| {
        let (game_id, creator, opponent) = setup_new_game();

        let moves = vec![
            (creator, 0, 0, Card::new(5, 3, 2, 4)),
            (opponent, 0, 1, Card::new(2, 4, 5, 3)),
            (creator, 1, 0, Card::new(4, 3, 1, 6)),
            (opponent, 1, 1, Card::new(3, 5, 2, 4)),
            (creator, 2, 0, Card::new(5, 3, 2, 4)),
            (opponent, 2, 1, Card::new(2, 4, 5, 3)),
            (creator, 3, 0, Card::new(4, 3, 1, 6)),
            (opponent, 3, 1, Card::new(3, 5, 2, 4)),
            (creator, 0, 2, Card::new(5, 3, 2, 4)),
            (opponent, 1, 2, Card::new(2, 4, 5, 3)),
        ];

        for (player, x, y, card) in &moves {
            log::debug!(
                "Player {} is attempting to play at ({}, {}) with card: {:?}",
                player,
                x,
                y,
                card
            );
            assert_ok!(Eterra::play_turn(
                frame_system::RawOrigin::Signed(*player).into(),
                game_id,
                *x,
                *y,
                card.clone()
            ));
        }

        // Assert that GameFinished event is emitted
        let events = frame_system::Pallet::<Test>::events();
        let game_finished_event_found = events.iter().any(|record| match record.event {
            RuntimeEvent::Eterra(crate::Event::GameFinished { game_id: event_game_id, winner: event_winner }) => {
                log::debug!("GameFinished event detected: {:?}, Winner: {:?}", event_game_id, event_winner);
                event_game_id == game_id
            }
            _ => false,
        });
        assert!(game_finished_event_found, "Expected GameFinished event was not found");

        log::debug!("Full game simulation completed and GameFinished event detected.");
    });
}


#[test]
fn play_out_of_turn_fails() {
    init_logger();
    new_test_ext().execute_with(|| {
        // Set up a new game
        let (game_id, creator, opponent) = setup_new_game();

        // First turn: the creator plays a card
        let card = Card::new(5, 3, 2, 4);
        assert_ok!(Eterra::play_turn(
            frame_system::RawOrigin::Signed(creator).into(),
            game_id,
            1,
            1,
            card.clone()
        ));

        // Second turn: the creator attempts to play again (out of turn)
        let another_card = Card::new(3, 4, 1, 2);
        let result = Eterra::play_turn(
            frame_system::RawOrigin::Signed(creator).into(),
            game_id,
            1,
            2,
            another_card
        );

        // Assert that the play fails with `NotYourTurn`
        assert_noop!(result, crate::Error::<Test>::NotYourTurn);

        // Confirm the opponent can play their turn
        let opponent_card = Card::new(2, 4, 5, 3);
        assert_ok!(Eterra::play_turn(
            frame_system::RawOrigin::Signed(opponent).into(),
            game_id,
            1,
            2,
            opponent_card
        ));

        log::debug!("Test completed: A player cannot play out of turn.");
    });
}
