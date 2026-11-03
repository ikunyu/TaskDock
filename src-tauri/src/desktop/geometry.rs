use super::settings::Placement;
#[derive(Clone, Debug, PartialEq)]
pub struct WorkArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub primary: bool,
}
/// Ephemeral compact geometry. It is never persisted as window_state; that
/// record continues to hold the normal expanded dimensions.
pub fn fit_compact(p: &Placement, areas: &[WorkArea]) -> Option<Placement> {
    let usable: Vec<_> = areas
        .iter()
        .filter(|a| a.width > 0 && a.height > 0 && a.scale.is_finite() && a.scale > 0.0)
        .collect();
    let area = usable
        .iter()
        .copied()
        .find(|a| {
            p.x >= a.x
                && p.y >= a.y
                && (p.x as i64) < a.x as i64 + a.width as i64
                && (p.y as i64) < a.y as i64 + a.height as i64
        })
        .or_else(|| usable.iter().copied().find(|a| a.primary))
        .or_else(|| usable.first().copied())?;
    let width = p
        .width
        .clamp(120.0, 480.0)
        .min(area.width as f64 / area.scale);
    let height = p
        .height
        .clamp(44.0, 600.0)
        .min(area.height as f64 / area.scale);
    let max_x = (area.x as i64 + area.width as i64 - (width * area.scale).ceil() as i64)
        .max(area.x as i64) as i32;
    let max_y = (area.y as i64 + area.height as i64 - (height * area.scale).ceil() as i64)
        .max(area.y as i64) as i32;
    Some(Placement {
        label: "widget".into(),
        x: p.x.clamp(area.x, max_x),
        y: p.y.clamp(area.y, max_y),
        width,
        height,
        scale: area.scale,
    })
}
pub fn defaults(label: &str) -> (f64, f64, f64, f64) {
    match label {
        "widget" => (180.0, 240.0, 160.0, 120.0),
        "quick-entry" => (560.0, 340.0, 420.0, 320.0),
        _ => (1120.0, 780.0, 760.0, 600.0),
    }
}
/// Positions are physical desktop coordinates. Sizes are logical pixels.
/// Recompute physical bounds using the *current target monitor's* scale.
pub fn fit(label: &str, saved: Option<&Placement>, areas: &[WorkArea]) -> Option<Placement> {
    let saved = saved.filter(|p| p.valid() && p.label == label);
    let area = saved
        .and_then(|p| {
            areas.iter().find(|a| {
                p.x >= a.x
                    && p.y >= a.y
                    && (p.x as i64) < a.x as i64 + a.width as i64
                    && (p.y as i64) < a.y as i64 + a.height as i64
            })
        })
        .or_else(|| areas.iter().find(|a| a.primary))
        .or_else(|| areas.first())?;
    let (dw, dh, minw, minh) = defaults(label);
    let (maxw, maxh) = if label == "widget" {
        (600.0, 800.0)
    } else {
        (10000.0, 10000.0)
    };
    let width = saved
        .map_or(dw, |p| p.width)
        .clamp(minw, maxw)
        .min(area.width as f64 / area.scale);
    // Reserve physical title bar/borders within work area.
    let height = saved
        .map_or(dh, |p| p.height)
        .clamp(minh, maxh)
        .min((area.height as f64 - 40.0 * area.scale).max(100.0) / area.scale);
    let pw = (width * area.scale).ceil() as i32 + 16;
    let ph = ((height + 40.0) * area.scale).ceil() as i32;
    let maxx = (area.x as i64 + area.width as i64 - pw as i64).max(area.x as i64) as i32;
    let maxy = (area.y as i64 + area.height as i64 - ph as i64).max(area.y as i64) as i32;
    let on_target = saved.is_some_and(|p| {
        p.x >= area.x
            && p.y >= area.y
            && (p.x as i64) < area.x as i64 + area.width as i64
            && (p.y as i64) < area.y as i64 + area.height as i64
    });
    let (x, y) = if on_target {
        let p = saved.unwrap();
        (p.x.clamp(area.x, maxx), p.y.clamp(area.y, maxy))
    } else if label == "widget" {
        ((maxx - 24).max(area.x), (area.y + 24).min(maxy))
    } else {
        (area.x + (maxx - area.x) / 2, area.y + (maxy - area.y) / 2)
    };
    Some(Placement {
        label: label.into(),
        x,
        y,
        width,
        height,
        scale: area.scale,
    })
}
