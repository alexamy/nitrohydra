use std::process::Command;

#[derive(Debug, Clone)]
pub struct Monitor {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub x: u32,
    pub y: u32,
}

/// Detect connected monitors by parsing `xrandr --query`.
/// Returns monitors sorted left-to-right (by x offset).
pub fn detect() -> Result<Vec<Monitor>, String> {
    let output = Command::new("xrandr")
        .arg("--query")
        .output()
        .map_err(|e| format!("failed to run xrandr: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("xrandr failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut monitors = Vec::new();

    for line in stdout.lines() {
        // Match lines like: "HDMI-0 connected 3840x2160+3840+0 ..."
        // or "DP-4 connected primary 3840x2160+0+0 ..."
        if !line.contains(" connected ") || line.contains(" disconnected ") {
            continue;
        }

        // Find the geometry token: WxH+X+Y
        let Some(geom) = line.split_whitespace().find(|tok| {
            tok.contains('x') && tok.contains('+')
        }) else {
            continue;
        };

        let name = line.split_whitespace().next().unwrap_or("").to_string();

        if let Some(mon) = parse_geometry(&name, geom) {
            monitors.push(mon);
        }
    }

    if monitors.is_empty() {
        return Err("no connected monitors found".into());
    }

    monitors.sort_by_key(|m| m.x);
    Ok(monitors)
}

fn parse_geometry(name: &str, geom: &str) -> Option<Monitor> {
    // Format: WxH+X+Y
    let (res, offsets) = geom.split_once('+')?;
    let (w, h) = res.split_once('x')?;
    let (x, y) = offsets.split_once('+')?;

    Some(Monitor {
        name: name.to_string(),
        width: w.parse().ok()?,
        height: h.parse().ok()?,
        x: x.parse().ok()?,
        y: y.parse().ok()?,
    })
}
