use game_core::{
    AiController, BuildingKind, FactionId, GameInput, InputBus, MatchConfig, SimWorld,
    VictoryState,
};

#[test]
fn duel_vs_ai_reaches_resolution_or_progresses() {
    let config = MatchConfig::duel_vs_ai();
    let mut world = SimWorld::new(config);
    let mut bus = InputBus::default();
    let mut ai = [AiController::new(), AiController::new()];

    for _ in 0..20 * 90 {
        bus.clear();
        let human = ai[0].decide(&world, FactionId(0));
        bus.set(FactionId(0), human);
        let d = ai[1].decide(&world, FactionId(1));
        bus.set(FactionId(1), d);
        let r = world.tick(&bus);
        if matches!(r.victory, VictoryState::TeamWon(_)) {
            return;
        }
    }
    assert!(
        world.buildings.len() >= 2,
        "expected buildings, got {}",
        world.buildings.len()
    );
    assert!(
        world.units.len() > 5,
        "expected units from buildings, got {}",
        world.units.len()
    );
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

#[test]
fn starting_gold_and_income_helper() {
    let world = SimWorld::new(MatchConfig::duel_vs_ai());
    assert!((world.gold[0] - 15.0).abs() < 0.01);
    assert!((world.income_per_sec(FactionId(0)) - 8.0).abs() < 0.01);
}

#[test]
fn place_rejects_without_gold() {
    let mut world = SimWorld::new(MatchConfig::duel_vs_ai());
    world.gold[0] = 100.0;
    let site = world
        .find_build_site(FactionId(0), BuildingKind::Barracks)
        .expect("site near castle");
    world.gold[0] = 5.0;
    assert!(!world.can_place(FactionId(0), BuildingKind::Barracks, site.0, site.1));
    let mut bus = InputBus::default();
    bus.set(
        FactionId(0),
        GameInput::place(BuildingKind::Barracks, site.0, site.1),
    );
    let r = world.tick(&bus);
    assert_eq!(r.buildings_placed, 0);
}
