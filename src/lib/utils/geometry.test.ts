import { describe, it, expect } from 'vitest';
import {
  anchorPosition,
  closestMonitor,
  ensureOnScreen,
  growsUpward,
  isCorner,
  monitorAt,
  panelGoesLeft,
  placeMenu,
  widgetOrigin,
  windowOrigin,
  type MonitorArea,
} from './geometry';

// A real two-screen layout: 1440p primary with a 48 px taskbar, and a 1080p
// screen to its left, shifted 200 px down (so it has negative coordinates).
const PRIMARY: MonitorArea = {
  bounds:   { x: 0, y: 0, width: 2560, height: 1440 },
  workArea: { x: 0, y: 0, width: 2560, height: 1392 },
};
const LEFT: MonitorArea = {
  bounds:   { x: -1920, y: 200, width: 1920, height: 1080 },
  workArea: { x: -1920, y: 200, width: 1920, height: 1032 },
};
const WIDGET = { width: 320, height: 330 };

describe('anchorPosition', () => {
  it('puts the widget 12 px inside each corner of the work area', () => {
    const area = PRIMARY.workArea;
    expect(anchorPosition('top-left', area, WIDGET, 12)).toEqual({ x: 12, y: 12 });
    expect(anchorPosition('top-right', area, WIDGET, 12)).toEqual({ x: 2228, y: 12 });
    expect(anchorPosition('bottom-left', area, WIDGET, 12)).toEqual({ x: 12, y: 1050 });
    expect(anchorPosition('bottom-right', area, WIDGET, 12)).toEqual({ x: 2228, y: 1050 });
  });

  it('uses the height of the widget itself, so a shorter widget sits lower', () => {
    const short = { width: 320, height: 100 };
    expect(anchorPosition('bottom-right', PRIMARY.workArea, short, 12)).toEqual({ x: 2228, y: 1280 });
  });

  it('is relative to the origin of a monitor that is not at 0,0', () => {
    expect(anchorPosition('top-left', LEFT.workArea, WIDGET, 12)).toEqual({ x: -1908, y: 212 });
    expect(anchorPosition('bottom-right', LEFT.workArea, WIDGET, 12)).toEqual({ x: -332, y: 890 });
  });
});

describe('monitorAt', () => {
  it('returns the monitor whose bounds contain the point', () => {
    expect(monitorAt([PRIMARY, LEFT], { x: 100, y: 100 })).toBe(PRIMARY);
    expect(monitorAt([PRIMARY, LEFT], { x: -100, y: 300 })).toBe(LEFT);
  });

  it('includes the top-left edge and excludes the bottom-right edge', () => {
    expect(monitorAt([PRIMARY], { x: 0, y: 0 })).toBe(PRIMARY);
    expect(monitorAt([PRIMARY], { x: 2560, y: 0 })).toBeNull();
    expect(monitorAt([PRIMARY], { x: 0, y: 1440 })).toBeNull();
  });

  it('returns null in the gap between two monitors', () => {
    // Left of the primary screen, but above the top of the shifted left screen.
    expect(monitorAt([PRIMARY, LEFT], { x: -100, y: 100 })).toBeNull();
  });
});

describe('closestMonitor', () => {
  it('prefers the monitor that contains the point', () => {
    expect(closestMonitor([PRIMARY, LEFT], { x: -100, y: 300 })).toBe(LEFT);
  });

  it('falls back to the nearest monitor for an off-screen point', () => {
    expect(closestMonitor([PRIMARY, LEFT], { x: 3000, y: 700 })).toBe(PRIMARY);
    expect(closestMonitor([PRIMARY, LEFT], { x: -5000, y: 300 })).toBe(LEFT);
  });

  it('returns null when no monitor is known', () => {
    expect(closestMonitor([], { x: 0, y: 0 })).toBeNull();
  });
});

describe('ensureOnScreen', () => {
  it('keeps a position whose centre is on a monitor', () => {
    expect(ensureOnScreen({ x: 2228, y: 12 }, WIDGET, [PRIMARY, LEFT])).toEqual({ x: 2228, y: 12 });
  });

  it('keeps a widget that only partly overhangs the edge', () => {
    // Centre is at x = 60: still on the primary screen.
    expect(ensureOnScreen({ x: -100, y: 100 }, WIDGET, [PRIMARY])).toEqual({ x: -100, y: 100 });
  });

  it('pulls a widget saved on an unplugged screen back inside the remaining one', () => {
    // Was on the left screen; only the primary is connected now.
    expect(ensureOnScreen({ x: -1500, y: 400 }, WIDGET, [PRIMARY])).toEqual({ x: 0, y: 400 });
    // Was on a screen to the right that no longer exists.
    expect(ensureOnScreen({ x: 4000, y: 500 }, WIDGET, [PRIMARY])).toEqual({ x: 2240, y: 500 });
  });

  it('keeps the whole widget above the taskbar when it was saved too low', () => {
    expect(ensureOnScreen({ x: 100, y: 3000 }, WIDGET, [PRIMARY])).toEqual({ x: 100, y: 1062 });
  });

  it('pulls into the nearest monitor when several are connected', () => {
    expect(ensureOnScreen({ x: -5000, y: 300 }, WIDGET, [PRIMARY, LEFT])).toEqual({ x: -1920, y: 300 });
  });

  it('aligns to the top-left when the widget is larger than the work area', () => {
    const tiny: MonitorArea = {
      bounds:   { x: 0, y: 0, width: 300, height: 200 },
      workArea: { x: 0, y: 0, width: 300, height: 200 },
    };
    expect(ensureOnScreen({ x: 900, y: 900 }, WIDGET, [tiny])).toEqual({ x: 0, y: 0 });
  });

  it('leaves the position alone when no monitor is known', () => {
    expect(ensureOnScreen({ x: 4000, y: 500 }, WIDGET, [])).toEqual({ x: 4000, y: 500 });
  });
});

describe('windowOrigin / widgetOrigin', () => {
  it('shifts the window left by the panel width when the widget is 218 CSS px inside it', () => {
    expect(windowOrigin({ x: 2228, y: 12 }, { x: 218, y: 0 }, 1)).toEqual({ x: 2010, y: 12 });
  });

  it('converts the CSS offset to physical pixels with the scale factor', () => {
    expect(windowOrigin({ x: 2228, y: 12 }, { x: 218, y: 0 }, 1.5)).toEqual({ x: 1901, y: 12 });
    expect(windowOrigin({ x: 100, y: 1050 }, { x: 0, y: 232 }, 1.25)).toEqual({ x: 100, y: 760 });
  });

  it('recovers the widget position from an expanded window (panel on the left)', () => {
    // The bug this guards: saving the window position (2010) instead of the widget's (2228).
    expect(widgetOrigin({ x: 2010, y: 12 }, { x: 218, y: 0 }, 1)).toEqual({ x: 2228, y: 12 });
    expect(widgetOrigin({ x: 1901, y: 12 }, { x: 218, y: 0 }, 1.5)).toEqual({ x: 2228, y: 12 });
  });

  it('is the window position itself when the widget fills the window', () => {
    expect(widgetOrigin({ x: 640, y: 480 }, { x: 0, y: 0 }, 1.5)).toEqual({ x: 640, y: 480 });
  });
});

describe('panelGoesLeft', () => {
  // The panel adds 218 px (210 + 8 gap) to the right of a 320 px widget.
  it('is false while widget + panel fit before the right edge', () => {
    expect(panelGoesLeft(12, 320, 218, PRIMARY.workArea)).toBe(false);
    expect(panelGoesLeft(2022, 320, 218, PRIMARY.workArea)).toBe(false); // ends exactly at 2560
  });

  it('is true as soon as the panel would cross the right edge', () => {
    expect(panelGoesLeft(2023, 320, 218, PRIMARY.workArea)).toBe(true);
    expect(panelGoesLeft(2228, 320, 218, PRIMARY.workArea)).toBe(true);
  });

  it('uses the right edge of the monitor the widget is on', () => {
    // Right edge of the left screen is x = 0.
    expect(panelGoesLeft(-332, 320, 218, LEFT.workArea)).toBe(true);
    expect(panelGoesLeft(-1908, 320, 218, LEFT.workArea)).toBe(false);
  });
});

describe('growsUpward', () => {
  it('is true when the widget centre is in the lower half of the work area', () => {
    expect(growsUpward(1050, 330, PRIMARY.workArea)).toBe(true);
    expect(growsUpward(890, 330, LEFT.workArea)).toBe(true);
  });

  it('is false in the upper half', () => {
    expect(growsUpward(12, 330, PRIMARY.workArea)).toBe(false);
    expect(growsUpward(212, 330, LEFT.workArea)).toBe(false);
  });
});

describe('placeMenu', () => {
  const MENU = { width: 230, height: 260 };
  const VIEW = { width: 320, height: 330 };

  it('opens at the cursor when there is room below and to the right', () => {
    expect(placeMenu({ x: 20, y: 20 }, MENU, VIEW)).toEqual({ x: 20, y: 20 });
  });

  it('flips to the left of the cursor near the right edge', () => {
    expect(placeMenu({ x: 300, y: 20 }, MENU, VIEW)).toEqual({ x: 70, y: 20 });
  });

  it('flips above the cursor near the bottom edge', () => {
    expect(placeMenu({ x: 20, y: 300 }, MENU, VIEW)).toEqual({ x: 20, y: 40 });
  });

  it('clamps inside the viewport when neither side has room', () => {
    expect(placeMenu({ x: 100, y: 100 }, MENU, VIEW)).toEqual({ x: 0, y: 0 });
  });

  it('never returns a negative position when the menu is taller than the viewport', () => {
    expect(placeMenu({ x: 10, y: 10 }, MENU, { width: 320, height: 200 })).toEqual({ x: 10, y: 0 });
  });
});

describe('isCorner', () => {
  it('accepts the four corners', () => {
    expect(isCorner('top-left')).toBe(true);
    expect(isCorner('bottom-right')).toBe(true);
  });

  it('rejects anything else read from the config file', () => {
    expect(isCorner('center')).toBe(false);
    expect(isCorner(null)).toBe(false);
    expect(isCorner(3)).toBe(false);
  });
});
