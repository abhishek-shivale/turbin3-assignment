mod helpers;

use helpers::*;


#[test]
fn init_creates_stake_state_with_correct_fields() {
    let mut svm = init_svm();
    let setup = setup_collection(&mut svm, DEFAULT_REWARDS_BPS, DEFAULT_FREEZE_PERIOD_DAYS);

    let state = fetch_stake_state(&svm, &setup.stake_state);
    assert_eq!(state.rewards_bps, DEFAULT_REWARDS_BPS);
    assert_eq!(state.freeze_period, DEFAULT_FREEZE_PERIOD_DAYS);
    assert_ne!(state.bump, 0);
    assert_ne!(state.reward_bump, 0);
}