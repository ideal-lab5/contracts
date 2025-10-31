"use client";

import React, { createContext, useContext, useState } from "react";
import { useEffect} from "react";
import { usePolkadot } from "./PolkadotContext";
import { BN, BN_ONE } from "@polkadot/util";
import { hexToString } from '@polkadot/util';

export type RiskLevel = "low" | "medium" | "high";

export interface GameState {
  mode: "manual";
  betAmount: number;
  risk: RiskLevel;
  rows: number;
  isRunning: boolean;
  balance: number;
  sound: boolean;
}

interface GameContextType {
  gameState: GameState;
  updateGameState: (updates: Partial<GameState>) => void;
  isReady: boolean;
}

const defaultState: GameState = {
  mode: "manual",
  betAmount: 1,
  risk: "low",
  rows: 8,
  isRunning: false,
  balance: 0,
  sound: true,
};

const GameContext = createContext<GameContextType | undefined>(undefined);
export const GameProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const [gameState, setGameState] = useState<GameState>(defaultState);
  const [isReady, setIsReady] = useState(false);

  const updateGameState = (updates: Partial<GameState>) => {
    setGameState((prev) => ({ ...prev, ...updates }));
  };

  console.log("calling usePolkadot()")

  const { idncClient, alice, contract, loading } = usePolkadot();

  useEffect(() => {
    if(!idncClient || !alice || !contract) return;
    const setBalance = async () => {
    if (loading) {
      console.log("Still loading...")
      return;
    }
      if (!idncClient || !alice || !contract) {
        console.log("Something wasn't ready")
        return;
      }

      const { data: { free } } = await idncClient.query.system.account(alice.address) as any;
  
      console.log('Free balance:', free.toHuman());

      // This sets the scale of the balance. Do not use as provided.
      const decimals = idncClient.registry.chainDecimals[0];
      const balanceFormatted = free.div(new BN(10).pow(new BN(decimals))).toNumber();

      updateGameState({balance: balanceFormatted});

      setIsReady(true)
    };

    setBalance();
  }, [idncClient, alice, contract, loading]);

  return (
    <GameContext.Provider value={{ gameState, updateGameState, isReady }}>
      {children}
    </GameContext.Provider>
  );
};

export const useGameContext = (): GameContextType => {
  const context = useContext(GameContext);
  if (!context)
    throw new Error("useGameContext must be used within GameProvider");
  return context;
};
