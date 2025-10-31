export interface GameState {
  mode: "manual";
  betAmount: number;
  risk: "low" | "medium" | "high";
  rows: number;
  isRunning: boolean;
  balance: number;
  sound: boolean;
}

export interface Multiplier {
  value: number;
  color: string;
}

export type RiskLevel = "low" | "medium" | "high";
