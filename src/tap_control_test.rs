use super::*;

#[test]
fn test_new_controller_starts_at_most_conservative_tap() {
    // Arrange & Act
    let controller = TapController::new();

    // Assert
    assert_eq!(controller.current(), TapPosition::Tap4);
}

#[test]
fn test_high_rating_steps_one_position_toward_target() {
    // Arrange
    let mut controller = TapController::new();

    // Act
    let actual = controller.on_rating(2500.0);

    // Assert -- target is Tap1, but a single tick only moves one step from Tap4.
    assert_eq!(actual, TapPosition::Tap3);
}

#[test]
fn test_controller_converges_to_target_over_multiple_ticks() {
    // Arrange
    let mut controller = TapController::new();

    // Act
    controller.on_rating(2500.0);
    controller.on_rating(2500.0);
    let actual = controller.on_rating(2500.0);

    // Assert -- Tap4 -> Tap3 -> Tap2 -> Tap1 over three ticks.
    assert_eq!(actual, TapPosition::Tap1);
}

#[test]
fn test_low_rating_from_tap1_steps_down_gradually() {
    // Arrange -- drive the controller up to Tap1 first.
    let mut controller = TapController::new();
    controller.on_rating(2500.0);
    controller.on_rating(2500.0);
    controller.on_rating(2500.0);
    assert_eq!(controller.current(), TapPosition::Tap1);

    // Act
    let actual = controller.on_rating(100.0);

    // Assert -- target is Tap4, but a single tick only moves one step from Tap1.
    assert_eq!(actual, TapPosition::Tap2);
}

#[test]
fn test_rating_within_current_band_holds_position() {
    // Arrange -- drive the controller to Tap2.
    let mut controller = TapController::new();
    controller.on_rating(2500.0);
    let actual_after_first_step = controller.on_rating(2500.0);
    assert_eq!(actual_after_first_step, TapPosition::Tap2);

    // Act -- a rating still inside the Tap2 band shouldn't move it.
    let actual = controller.on_rating(1500.0);

    // Assert
    assert_eq!(actual, TapPosition::Tap2);
}

#[test]
fn test_target_for_rating_band_boundaries() {
    // Arrange & Act & Assert
    assert_eq!(target_for_rating(2000.0), TapPosition::Tap1);
    assert_eq!(target_for_rating(1999.9), TapPosition::Tap2);
    assert_eq!(target_for_rating(1400.0), TapPosition::Tap2);
    assert_eq!(target_for_rating(1399.9), TapPosition::Tap3);
    assert_eq!(target_for_rating(800.0), TapPosition::Tap3);
    assert_eq!(target_for_rating(799.9), TapPosition::Tap4);
    assert_eq!(target_for_rating(0.0), TapPosition::Tap4);
}

#[test]
fn test_tap_position_enum_sample_wire_values() {
    // Arrange & Act & Assert -- per ems/topic_structure_adr.md §6 EnumSample.
    assert_eq!(TapPosition::Tap1.as_str(), "TAP_1");
    assert_eq!(TapPosition::Tap2.as_str(), "TAP_2");
    assert_eq!(TapPosition::Tap3.as_str(), "TAP_3");
    assert_eq!(TapPosition::Tap4.as_str(), "TAP_4");
}
