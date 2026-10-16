use game_core::{
    AiController, FactionId, GameInput, InputBus, MatchConfig, SimWorld, UnitType, VictoryState,
};

#[test]
fn duel_vs_ai_reaches_resolution_or_progresses() {
    let config = MatchConfig::duel_vs_ai();
    let mut world = SimWorld::new(config);
    let mut bus = InputBus::default();
    let mut ai = [AiController::new(), AiController::new()];

    for _ in 0..20 * 60 {
        // 60s
        bus.clear();
        bus.set(FactionId(0), GameInput::produce(UnitType::Melee));
        let d = ai[1].decide(&world, FactionId(1));
        bus.set(FactionId(1), d);
        let r = world.tick(&bus);
        if matches!(r.victory, VictoryState::TeamWon(_)) {
            return;
        }
    }
    // Even without victory, we should have units and some economy
    assert!(world.units.len() > 10, "expected mass units, got {}", world.units.len());
    assert!(world.tick > 100);
}

#[test]
fn two_v_two_has_four_castles_and_alliances() {
    let config = MatchConfig::two_v_two_solo();
    let world = SimWorld::new(config);
    assert_eq!(world.castles.len(), 4);
    assert!(world.are_allied(FactionId(0), FactionId(1)));
    assert!(!world.are_allied(FactionId(0), FactionId(2)));
}
