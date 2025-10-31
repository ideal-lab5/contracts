export interface PlinkoItem {
  value: number;
  color: string;
}

export type PlinkoConfig = {
  [risk: string]: {
    [rows: number]: PlinkoItem[];
  };
};

export const CANVAS_WIDTH = 600;
export const CANVAS_HEIGHT = 450;
export const BALL_RADIUS = 10;
export const PEG_RADIUS = 4;
export const DROP_ZONES = 9;

/**
 * A sample config. Each array is the "base" set of multipliers
 * that will be mirrored in the GameBoard code.
 *
 * For example, if you have `rows=16` and `risk=high`,
 * then you get this base array:
 *  [
 *    { value: 0.3, color: "#f59e0b" },
 *    { value: 0.5, color: "#f59e0b" },
 *    { value: 1,   color: "#f97316" },
 *    { value: 1.5, color: "#f97316" },
 *    { value: 3,   color: "#ef4444" },
 *    { value: 5,   color: "#ef4444" },
 *    { value: 10,  color: "#ef4444" },
 *    { value: 25,  color: "#ef4444" },
 *    { value: 50,  color: "#ef4444" }
 *  ]
 *
 * Then in code, you'd mirror it to produce left->right symmetrical multipliers.
 */
export const PLINKO_CONFIG: PlinkoConfig = {
  low: {
    8: [
      { value: 0, color: "#f59e0b" },
      { value: 1, color: "#f59e0b" },
      { value: 2, color: "#f97316" },
      { value: 3, color: "#ef4444" },
      { value: 5, color: "#ef4444" },
    ],
    16: [
      { value: 0.3, color: "#f59e0b" },
      { value: 0.5, color: "#f59e0b" },
      { value: 1, color: "#f97316" },
      { value: 2, color: "#f97316" },
      { value: 5, color: "#ef4444" },
    ],
  },
  medium: {
    8: [
      { value: 1, color: "#f59e0b" },
      { value: 2, color: "#f97316" },
      { value: 5, color: "#ef4444" },
      { value: 10, color: "#ef4444" },
    ],
    16: [
      { value: 0.3, color: "#f59e0b" },
      { value: 0.7, color: "#f59e0b" },
      { value: 1, color: "#f97316" },
      { value: 2, color: "#f97316" },
      { value: 3, color: "#ef4444" },
    ],
  },
  high: {
    8: [
      { value: 0.5, color: "#f59e0b" },
      { value: 1.5, color: "#f97316" },
      { value: 3, color: "#ef4444" },
      { value: 10, color: "#ef4444" },
    ],
    16: [
      { value: 0.3, color: "#f59e0b" },
      { value: 0.5, color: "#f59e0b" },
      { value: 1, color: "#f97316" },
      { value: 1.5, color: "#f97316" },
      { value: 3, color: "#ef4444" },
    ],
  },
};
