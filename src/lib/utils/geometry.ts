/**
 * Pure window-geometry helpers. Positions are in physical pixels (what Tauri's
 * `PhysicalPosition` and monitor rects use) unless a name says "Css".
 */

export interface Point { x: number; y: number }
export interface Size { width: number; height: number }
export interface Rect extends Point, Size {}

export const CORNERS = ['top-left', 'top-right', 'bottom-left', 'bottom-right'] as const;
export type Corner = (typeof CORNERS)[number];

export interface MonitorArea {
  /** Full monitor rectangle. */
  bounds: Rect;
  /** Monitor rectangle minus the taskbar. */
  workArea: Rect;
}

export function isCorner(value: unknown): value is Corner {
  return typeof value === 'string' && (CORNERS as readonly string[]).includes(value);
}

/** Top-left of a widget of `size` sitting `margin` px inside `corner` of `area`. */
export function anchorPosition(corner: Corner, area: Rect, size: Size, margin: number): Point {
  const left = corner === 'top-left' || corner === 'bottom-left';
  const top  = corner === 'top-left' || corner === 'top-right';
  return {
    x: left ? area.x + margin : area.x + area.width  - size.width  - margin,
    y: top  ? area.y + margin : area.y + area.height - size.height - margin,
  };
}

function contains(r: Rect, p: Point): boolean {
  return p.x >= r.x && p.x < r.x + r.width && p.y >= r.y && p.y < r.y + r.height;
}

/** Squared distance from `p` to the nearest point of `r` (0 when inside). */
function distanceSq(r: Rect, p: Point): number {
  const dx = Math.max(r.x - p.x, 0, p.x - (r.x + r.width));
  const dy = Math.max(r.y - p.y, 0, p.y - (r.y + r.height));
  return dx * dx + dy * dy;
}

/** The monitor whose bounds contain `p`, or null. */
export function monitorAt<M extends MonitorArea>(monitors: M[], p: Point): M | null {
  return monitors.find((m) => contains(m.bounds, p)) ?? null;
}

/** The monitor containing `p`, else the nearest one; null when the list is empty. */
export function closestMonitor<M extends MonitorArea>(monitors: M[], p: Point): M | null {
  let best: M | null = null;
  let bestDistance = Infinity;
  for (const m of monitors) {
    const d = distanceSq(m.bounds, p);
    if (d < bestDistance) { best = m; bestDistance = d; }
  }
  return best;
}

/**
 * Position to restore a saved widget at. Unchanged while the widget's centre is
 * on a connected monitor; otherwise (screen unplugged, resolution changed) the
 * widget is pulled fully inside the nearest monitor's work area.
 */
export function ensureOnScreen(pos: Point, size: Size, monitors: MonitorArea[]): Point {
  const centre = { x: pos.x + size.width / 2, y: pos.y + size.height / 2 };
  if (monitorAt(monitors, centre)) return pos;
  const target = closestMonitor(monitors, centre);
  if (!target) return pos;
  const area = target.workArea;
  return {
    x: Math.max(area.x, Math.min(pos.x, area.x + area.width  - size.width)),
    y: Math.max(area.y, Math.min(pos.y, area.y + area.height - size.height)),
  };
}

/** Window top-left that puts the widget at `widget`, given the widget's offset inside the window. */
export function windowOrigin(widget: Point, offsetCss: Point, scale: number): Point {
  return {
    x: widget.x - Math.round(offsetCss.x * scale),
    y: widget.y - Math.round(offsetCss.y * scale),
  };
}

/** Inverse of `windowOrigin`: where the widget is on screen for a given window position. */
export function widgetOrigin(window: Point, offsetCss: Point, scale: number): Point {
  return {
    x: window.x + Math.round(offsetCss.x * scale),
    y: window.y + Math.round(offsetCss.y * scale),
  };
}

/** True when a side panel adding `span` px to the right of the widget would cross the right edge of `area`. */
export function panelGoesLeft(widgetX: number, widgetWidth: number, span: number, area: Rect): boolean {
  return widgetX + widgetWidth + span > area.x + area.width;
}

/** True when the widget's centre is in the lower half of `area`: extra content should then extend upward. */
export function growsUpward(widgetY: number, widgetHeight: number, area: Rect): boolean {
  return widgetY + widgetHeight / 2 > area.y + area.height / 2;
}

/**
 * Top-left for a popup opened at `click`: flips to the other side of the cursor
 * when it would overflow the viewport, then clamps so it never leaves it.
 */
export function placeMenu(click: Point, menu: Size, viewport: Size): Point {
  const place = (at: number, length: number, limit: number) => {
    const raw = at + length > limit ? at - length : at;
    return Math.max(0, Math.min(raw, limit - length));
  };
  return {
    x: place(click.x, menu.width, viewport.width),
    y: place(click.y, menu.height, viewport.height),
  };
}
