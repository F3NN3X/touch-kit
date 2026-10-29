//! Normalised digitizer coordinates → panel pixels.
//!
//! Order: rotate (clockwise) → flip → scale to px, or, when calibrated, rotate → flip → affine fit.

use crate::config::{CalPoint, PanelConfig, Rotation};
use crate::Error;

#[derive(Clone, Debug)]
pub struct Mapper {
    w: f32,
    h: f32,
    rotation: Rotation,
    flip_x: bool,
    flip_y: bool,
    /// px = a·u + b·v + c, rows for x and y.
    affine: Option<[[f32; 3]; 2]>,
}

impl Mapper {
    pub fn new(cfg: &PanelConfig) -> Result<Self, Error> {
        if cfg.width_px == 0 || cfg.height_px == 0 {
            return Err(Error::Config("panel size must be non-zero".into()));
        }
        let affine = cfg.calibration.as_ref().map(fit_affine).transpose()?;
        Ok(Self {
            w: cfg.width_px as f32,
            h: cfg.height_px as f32,
            rotation: cfg.rotation,
            flip_x: cfg.flip_x,
            flip_y: cfg.flip_y,
            affine,
        })
    }

    /// Normalised (0..1) touch → panel px. 1.0 maps to the last pixel (w−1, h−1).
    pub fn map(&self, x: f32, y: f32) -> (f32, f32) {
        let (mut u, mut v) = match self.rotation {
            Rotation::R0 => (x, y),
            Rotation::R90 => (1.0 - y, x),
            Rotation::R180 => (1.0 - x, 1.0 - y),
            Rotation::R270 => (y, 1.0 - x),
        };
        if self.flip_x {
            u = 1.0 - u;
        }
        if self.flip_y {
            v = 1.0 - v;
        }
        match &self.affine {
            Some([ax, ay]) => (
                (ax[0] * u + ax[1] * v + ax[2]).clamp(0.0, self.w - 1.0),
                (ay[0] * u + ay[1] * v + ay[2]).clamp(0.0, self.h - 1.0),
            ),
            None => (u * (self.w - 1.0), v * (self.h - 1.0)),
        }
    }
}

/// How well four calibration pairs fit one affine transform: the RMS distance, in panel px, between
/// each pair's panel point and where the fit maps its touch. Err if the points can't be fitted.
pub fn calibration_error(pts: &[CalPoint; 4]) -> Result<f32, Error> {
    let [ax, ay] = fit_affine(pts)?;
    let sq: f32 = pts
        .iter()
        .map(|p| {
            let (u, v) = (p.touch[0], p.touch[1]);
            let dx = ax[0] * u + ax[1] * v + ax[2] - p.panel[0];
            let dy = ay[0] * u + ay[1] * v + ay[2] - p.panel[1];
            dx * dx + dy * dy
        })
        .sum();
    Ok((sq / 4.0).sqrt())
}

/// Least-squares affine fit over the four calibration pairs (normal equations, Cramer's rule).
fn fit_affine(pts: &[CalPoint; 4]) -> Result<[[f32; 3]; 2], Error> {
    let mut m = [[0f64; 3]; 3];
    let mut bx = [0f64; 3];
    let mut by = [0f64; 3];
    for p in pts {
        let r = [p.touch[0] as f64, p.touch[1] as f64, 1.0];
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] += r[i] * r[j];
            }
            bx[i] += r[i] * p.panel[0] as f64;
            by[i] += r[i] * p.panel[1] as f64;
        }
    }
    let det = det3(&m);
    if det.abs() < 1e-9 {
        return Err(Error::Config("calibration points are collinear or repeated".into()));
    }
    let solve = |b: &[f64; 3]| {
        let mut out = [0f32; 3];
        for (k, o) in out.iter_mut().enumerate() {
            let mut mk = m;
            for row in 0..3 {
                mk[row][k] = b[row];
            }
            *o = (det3(&mk) / det) as f32;
        }
        out
    };
    Ok([solve(&bx), solve(&by)])
}

fn det3(m: &[[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}
