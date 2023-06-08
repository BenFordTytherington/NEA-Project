//! A module containing various interpolation methods: Linear, Lanczos and Hermite spline

use std::f32::consts::PI;

/// Linearly interpolates between `a` and `b` by parameter `t`
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

/// Sinc function defined as sin (pi x) / (pi x)
/// Defined as 1 at x = 0 (division by zero otherwise)
fn sinc(x: f32) -> f32 {
    if x == 0.0 {
        1.0
    } else {
        let px = PI * x;
        px.sin() / px
    }
}

/// Sinc function with length stretched to map range -1 <= x <= 1 to range -a <= x <= a
/// Used as the kernel in Lanczos interpolation
pub fn lanczos_window(x: f32, a: f32) -> f32 {
    if x.abs() <= a {
        sinc(x) * sinc(x / a)
    } else {
        0.0
    }
}

// The hermite basis functions

#[allow(missing_docs)]
fn h00(t: f32) -> f32 {
    (1.0 + 2.0 * t) * (1.0 - t).powi(2)
}

#[allow(missing_docs)]
fn h01(t: f32) -> f32 {
    t.powi(2) * (3.0 - 2.0 * t)
}

#[allow(missing_docs)]
fn h10(t: f32) -> f32 {
    t * (1.0 - t).powi(2)
}

#[allow(missing_docs)]
fn h11(t: f32) -> f32 {
    t.powi(2) * (t - 1.0)
}

/// Function which interpolates a value between the points p0 through p2, given a stretch factor and a t interpolant
pub fn hermite_interpolate(p0: f32, p1: f32, p2: f32, p3: f32, factor: f32, t: f32) -> f32 {
    // the gradient between the points 1 after and 1 before the sample with respect to time
    let m1 = (p2 - p0) * 0.5 * factor;
    // the gradient between the current sample and the sample 2 after it with respect to time.
    let m2 = (p3 - p1) * 0.5 * factor;

    // Calculating the interpolated value using the points, function values at interpolant t and the gradients for those points
    p1 * h00(t) + m1 * h10(t) + p2 * h01(t) + m2 * h11(t)
}
