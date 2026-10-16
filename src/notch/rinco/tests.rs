use rstest::rstest;

use super::*;

const MOODS: [Mood; 5] = [Mood::Done, Mood::Working, Mood::Error, Mood::Question, Mood::Needs];

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

#[test]
fn every_frame_is_a_sixteen_by_sixteen_grid_of_palette_colours() {
    let kit = kit().expect("the kit parses");
    for (name, animation) in &kit.states {
        for frame in &animation.frames {
            assert_eq!(frame.rows.len(), SIZE, "{name}");
            for row in &frame.rows {
                assert_eq!(row.chars().count(), SIZE, "{name}: {row}");
                assert!(row.chars().all(|c| c == '.' || colour(c).is_some()), "{name}: {row}");
            }
        }
    }
}

#[rstest]
#[case::working(Mood::Working, &[240, 240, 240, 240], true)]
#[case::needs(Mood::Needs, &[170, 170, 170, 450], true)]
#[case::question(Mood::Question, &[900, 650, 400, 650], true)]
#[case::done(Mood::Done, &[110, 150, 110, 150, 2200], false)]
#[case::error(Mood::Error, &[60, 60, 60, 60, 1800], false)]
fn each_state_keeps_the_designs_timing(#[case] mood: Mood, #[case] timing: &[u64], #[case] looping: bool) {
    let animation = &kit().expect("the kit parses").states[mood.key()];
    let found: Vec<u64> = animation.frames.iter().map(|f| f.ms).collect();

    assert_eq!((found.as_slice(), animation.looping), (timing, looping));
}

#[test]
fn a_loop_starts_over() {
    assert_eq!([0, 239, 240, 479, 480, 960].map(|n| index(Mood::Working, ms(n))), [0, 0, 1, 1, 2, 0].map(Some));
}

#[test]
fn a_one_shot_holds_its_last_frame() {
    assert_eq!([0, 110, 520, 2719, 60_000].map(|n| index(Mood::Done, ms(n))), [0, 1, 4, 4, 4].map(Some));
}

#[test]
fn every_state_rinco_shows_has_frames() {
    assert!(MOODS.iter().all(|m| frame(*m, ms(0)).is_some()));
}
