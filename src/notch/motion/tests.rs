use super::*;

const SMALL: Size = Size { width: 180.0, height: 32.0 };
const BIG: Size = Size { width: 380.0, height: 200.0 };

#[test]
fn the_spring_starts_still_and_settles_on_its_target() {
    assert_eq!((spring(0.0), spring(SETTLE.as_secs_f64())), (0.0, 1.0));
}

#[test]
fn it_barely_overshoots() {
    let peak = (1..300).map(|ms| spring(f64::from(ms) / 1000.0)).fold(0.0, f64::max);

    assert!(peak < 1.005, "peak {peak}");
}

#[test]
fn it_is_nearly_there_before_it_settles() {
    assert!((spring(0.25) - 1.0).abs() < 0.01);
}

#[test]
fn opening_grows_from_the_notch_and_fades_the_cards_in_late() {
    let motion = Motion { from: SMALL, to: BIG, opening: true, fresh: true };

    let start = motion.at(Duration::ZERO);
    let middle = motion.at(SETTLE / 4);
    let end = motion.at(SETTLE);

    assert_eq!((start.size, start.alpha, start.done), (SMALL, 0.0, false));
    assert!(middle.size.height > SMALL.height && middle.alpha < 0.5);
    assert_eq!((end.size, end.alpha, end.done), (BIG, 1.0, true));
}

#[test]
fn a_change_of_cards_springs_to_its_size_without_fading() {
    let motion = Motion { from: SMALL, to: BIG, opening: true, fresh: false };

    let middle = motion.at(SETTLE / 4);

    assert!(middle.size.height > SMALL.height && middle.size.height < BIG.height);
    assert_eq!(middle.alpha, 1.0);
}

#[test]
fn closing_fades_the_cards_out_before_it_shrinks_away() {
    let motion = Motion { from: BIG, to: SMALL, opening: false, fresh: false };

    let early = motion.at(SETTLE / 4);
    let end = motion.at(SETTLE);

    assert!(early.alpha < 0.5 && early.size.height > SMALL.height);
    assert_eq!((end.size, end.alpha, end.done), (SMALL, 0.0, true));
}
