//! The app's own movements, timed the way niri times its animations: the
//! motion the config gives an animation (or niri's default for it), the
//! config's slowdown, and none at all when animations are off.

use std::time::Duration;

use crate::animations::{Animation, Curve, Motion, Spring};
use crate::settings::{Reading, Setting};
use crate::store::Store;

/// A movement as niri would play `animation` with the config at hand.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Played {
    motion: Motion,
    /// The custom curve's control points, for a cubic bezier.
    bezier: [f32; 4],
    /// The config's `slowdown`: 2 plays everything at half speed.
    slowdown: f32,
    /// How long it takes, worked out once: an overdamped spring's is a
    /// search.
    duration: Duration,
}

/// How niri plays `animation` with `store` as its config: `None` when the
/// config turns it, or every animation, off. Without a config, as niri
/// plays it by default.
pub fn played(store: Option<&Store>, animation: Animation) -> Option<Played> {
    let name = animation.name();
    let Some(store) = store else {
        return Some(Played::new(animation.default_motion(), None, 1.0));
    };
    if !Setting::Animations.spec().is_on(store) || store.animation_off(name) {
        return None;
    }
    let slowdown = match Setting::AnimationsSlowdown.spec().read(store) {
        Reading::Value(Some(value)) => value.parse().unwrap_or(1.0),
        Reading::Value(None) | Reading::Switch(_) | Reading::OnOff(_) => 1.0,
    };
    let motion = if let Some(spring) = store.spring_params(name) {
        Motion::Spring(spring)
    } else if let Some(curve) = store
        .easing_curve(name)
        .as_deref()
        .and_then(Curve::from_name)
    {
        let default_ms = match animation.default_motion() {
            Motion::Easing { duration_ms, .. } => duration_ms,
            Motion::Spring(_) => 250,
        };
        Motion::Easing {
            curve,
            duration_ms: store.duration_ms(name).unwrap_or(default_ms),
        }
    } else {
        animation.default_motion()
    };
    Some(Played::new(motion, store.bezier_points(name), slowdown))
}

/// The longest the app moves anything, in seconds.
const MOST: f32 = 2.0;

impl Played {
    fn new(motion: Motion, bezier: Option<[f32; 4]>, slowdown: f32) -> Self {
        // niri treats a slowdown of 0 as no animation at all; a tiny one
        // keeps the arithmetic finite
        let slowdown = if slowdown.is_finite() {
            slowdown.max(0.001)
        } else {
            1.0
        };
        let own = match motion {
            Motion::Easing { duration_ms, .. } => {
                Duration::from_millis(duration_ms.into()).as_secs_f32()
            }
            Motion::Spring(spring) => Oscillator::new(spring).duration(),
        };
        // the app moves for two seconds at most: a slower movement plays
        // faster to end at rest, and a spring that never settles (no
        // damping) is cut off
        let slowdown = if own.is_finite() && own * slowdown > MOST {
            MOST / own
        } else {
            slowdown
        };
        Self {
            motion,
            bezier: bezier.unwrap_or([0.25, 0.1, 0.25, 1.0]),
            slowdown,
            duration: Duration::from_secs_f32((own * slowdown).clamp(0.0, MOST)),
        }
    }

    /// How long the movement takes, slowdown included.
    pub const fn duration(&self) -> Duration {
        self.duration
    }

    /// How far along the movement is `elapsed` after it started: 0 at the
    /// start, 1 at rest. An underdamped spring passes 1 on its way.
    pub fn at(&self, elapsed: Duration) -> f32 {
        if elapsed >= self.duration() {
            return 1.0;
        }
        let seconds = elapsed.as_secs_f32() / self.slowdown;
        match self.motion {
            Motion::Easing { curve, duration_ms } => {
                let length = Duration::from_millis(duration_ms.max(1).into()).as_secs_f32();
                ease(curve, self.bezier, (seconds / length).clamp(0.0, 1.0))
            }
            Motion::Spring(spring) => 1.0 - Oscillator::new(spring).offset(seconds),
        }
    }
}

/// niri's easing curves at `x` from 0 to 1.
fn ease(curve: Curve, bezier: [f32; 4], along: f32) -> f32 {
    match curve {
        Curve::Linear => along,
        Curve::EaseOutQuad => 1.0 - (1.0 - along).powi(2),
        Curve::EaseOutCubic => 1.0 - (1.0 - along).powi(3),
        // niri's own: 1 - 2^(-10x), which stops just short of 1
        Curve::EaseOutExpo => 1.0 - 2f32.powf(-10.0 * along),
        Curve::CubicBezier => cubic_bezier(bezier, along),
    }
}

/// A CSS cubic-bezier from (0, 0) to (1, 1) at `x`: the curve's parameter
/// found by bisection, which needs no derivative and always ends.
fn cubic_bezier([x1, y1, x2, y2]: [f32; 4], along: f32) -> f32 {
    // one coordinate of the curve at parameter `along`, from its two
    // control points' coordinates
    let at = |first: f32, second: f32, along: f32| {
        let rest = 1.0 - along;
        3.0 * rest * rest * along * first
            + 3.0 * rest * along * along * second
            + along * along * along
    };
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..40 {
        let middle = f32::midpoint(low, high);
        if at(x1, x2, middle) < along {
            low = middle;
        } else {
            high = middle;
        }
    }
    at(y1, y2, f32::midpoint(low, high))
}

/// How far from rest `spring` is `seconds` after it set off from 1 toward
/// 0: what the Animations page draws, as niri moves it.
pub fn spring_offset(spring: Spring, seconds: f32) -> f32 {
    Oscillator::new(spring).offset(seconds)
}

/// niri's spring moving from 1 to 0, mass 1, at rest at first.
// after niri's src/animation/spring.rs (oscillate and duration), itself
// after libadwaita's adw-spring-animation.c
struct Oscillator {
    /// Half the damping per mass (beta).
    beta: f32,
    /// The undamped angular frequency (omega0).
    omega: f32,
    epsilon: f32,
}

impl Oscillator {
    fn new(spring: Spring) -> Self {
        let stiffness = spring.stiffness.max(0.0);
        Self {
            beta: spring.damping_ratio.max(0.0) * stiffness.sqrt(),
            omega: stiffness.sqrt(),
            epsilon: spring.epsilon.max(0.0),
        }
    }

    /// How far from rest it is `t` seconds in: 1 at the start.
    fn offset(&self, seconds: f32) -> f32 {
        let (beta, omega) = (self.beta, self.omega);
        let envelope = (-beta * seconds).exp();
        if (beta - omega).abs() <= f32::EPSILON {
            envelope * beta.mul_add(seconds, 1.0)
        } else if beta < omega {
            let omega1 = omega.mul_add(omega, -(beta * beta)).sqrt();
            envelope * ((omega1 * seconds).cos() + beta / omega1 * (omega1 * seconds).sin())
        } else {
            // e^(-bt)(cosh(wt) + b/w sinh(wt)) as two decaying terms: cosh
            // and sinh alone overflow f32 long before the product is small
            let omega2 = beta.mul_add(beta, -(omega * omega)).sqrt();
            let ratio = beta / omega2;
            0.5 * (1.0 + ratio).mul_add(
                (-(beta - omega2) * seconds).exp(),
                (1.0 - ratio) * (-(beta + omega2) * seconds).exp(),
            )
        }
    }

    /// Seconds until it is at rest, within epsilon.
    fn duration(&self) -> f32 {
        if self.beta <= f32::EPSILON {
            return f32::INFINITY;
        }
        // the envelope's time to epsilon, which is the answer unless the
        // spring is overdamped and creeps in slower than its envelope
        let envelope = -self.epsilon.max(f32::MIN_POSITIVE).ln() / self.beta;
        if self.beta <= self.omega + f32::EPSILON {
            return envelope;
        }
        let mut seconds = envelope;
        while self.offset(seconds).abs() > self.epsilon && seconds < 10.0 {
            seconds += 0.001;
        }
        seconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spring(damping_ratio: f32, stiffness: f32, epsilon: f32) -> Played {
        Played::new(
            Motion::Spring(Spring {
                damping_ratio,
                stiffness,
                epsilon,
            }),
            None,
            1.0,
        )
    }

    #[test]
    fn a_movement_starts_at_zero_and_rests_at_one() {
        let motions = [
            spring(1.0, 800.0, 0.0001),
            spring(0.6, 1000.0, 0.001),
            spring(2.0, 300.0, 0.001),
            Played::new(
                Motion::Easing {
                    curve: Curve::EaseOutExpo,
                    duration_ms: 150,
                },
                None,
                1.0,
            ),
            Played::new(
                Motion::Easing {
                    curve: Curve::CubicBezier,
                    duration_ms: 300,
                },
                Some([0.05, 0.7, 0.1, 1.0]),
                1.0,
            ),
        ];
        for motion in motions {
            assert!(motion.at(Duration::ZERO).abs() < 1e-3, "{motion:?}");
            assert!(
                (motion.at(motion.duration()) - 1.0).abs() < 1e-6,
                "{motion:?}"
            );
        }
    }

    #[test]
    fn an_underdamped_spring_overshoots_and_a_critical_one_does_not() {
        // niri's config notification bounces; window movement does not
        let bouncy = spring(0.6, 1000.0, 0.001);
        let critical = spring(1.0, 800.0, 0.0001);
        let peak = |motion: Played| {
            (0..2000)
                .map(|ms| motion.at(Duration::from_millis(ms)))
                .fold(0.0_f32, f32::max)
        };
        assert!(peak(bouncy) > 1.05, "{}", peak(bouncy));
        assert!(peak(critical) <= 1.0 + 1e-4, "{}", peak(critical));
    }

    #[test]
    fn a_slowdown_past_the_longest_still_ends_at_rest() {
        let easing = Motion::Easing {
            curve: Curve::EaseOutCubic,
            duration_ms: 250,
        };
        let slow = Played::new(easing, None, 10.0);
        assert_eq!(slow.duration(), Duration::from_secs(2));
        let almost = slow.at(Duration::from_millis(1990));
        assert!(almost > 0.99, "{almost} just before the end");
    }

    #[test]
    fn a_stiff_overdamped_spring_stays_finite() {
        // niri accepts a damping ratio up to 10
        let creeping = spring(10.0, 1000.0, 0.0001);
        for millis in [100, 300, 1000, 1900] {
            let along = creeping.at(Duration::from_millis(millis));
            assert!((0.0..=1.0).contains(&along), "{along} at {millis} ms");
        }
        assert!(creeping.duration() > Duration::from_secs(1));
    }

    #[test]
    fn slowdown_stretches_the_time() {
        let normal = spring(1.0, 800.0, 0.0001);
        let slow = Played::new(normal.motion, None, 3.0);
        let ratio = slow.duration().as_secs_f32() / normal.duration().as_secs_f32();
        assert!((ratio - 3.0).abs() < 0.01, "{ratio}");
        let half = Duration::from_millis(50);
        assert!((slow.at(half * 3) - normal.at(half)).abs() < 1e-4);
    }

    #[test]
    fn the_spring_settles_when_niri_says_it_does() {
        // niri: the envelope time -ln(epsilon) / beta, beta = ratio *
        // sqrt(stiffness) for mass 1
        let motion = spring(0.6, 1000.0, 0.001);
        let expected = -(0.001_f32.ln()) / (0.6 * 1000_f32.sqrt());
        assert!((motion.duration().as_secs_f32() - expected).abs() < 1e-5);
    }

    #[test]
    fn the_config_turns_movements_off_or_slows_them() {
        let animation = Animation::ConfigNotificationOpenClose;
        let store = |text: &str| Store::from_text("config.kdl".into(), text).unwrap();
        assert!(played(Some(&store("")), animation).is_some());
        assert!(played(Some(&store("animations {\n    off\n}\n")), animation).is_none());
        let one_off = "animations {\n    config-notification-open-close {\n        off\n    }\n}\n";
        assert!(played(Some(&store(one_off)), animation).is_none());
        let slow = played(
            Some(&store("animations {\n    slowdown 2.0\n}\n")),
            animation,
        )
        .unwrap();
        let normal = played(None, animation).unwrap();
        assert!(slow.duration() > normal.duration());
        let curve = "animations {\n    config-notification-open-close {\n        duration-ms 300\n        curve \"linear\"\n    }\n}\n";
        let linear = played(Some(&store(curve)), animation).unwrap();
        // seconds in f32: within a millisecond of what the config says
        let off = linear.duration().abs_diff(Duration::from_millis(300));
        assert!(off < Duration::from_millis(1), "{off:?}");
        assert!((linear.at(Duration::from_millis(150)) - 0.5).abs() < 1e-3);
    }
}
