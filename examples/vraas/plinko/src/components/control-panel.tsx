"use client";

import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Input } from "@/components/ui/input";
import { Volume2, VolumeX } from "lucide-react";
import { useGameContext } from "@/context/GameContext";

interface ControlPanelProps {
  onSendBall: () => void;
}

export function ControlPanel({
  // onUpdateGameState,
  onSendBall,
}: ControlPanelProps) {
  const { gameState, updateGameState } = useGameContext();

  const handleBetChange = (value: string) => {
    const bet = Number.parseFloat(value);
    if (!isNaN(bet) && bet > 0) {
      updateGameState({ betAmount: bet });
    }
  };

  return (
    <div className='w-full md:w-72 p-4 md:p-6 bg-[#1a1b23] rounded-lg space-y-4 md:space-y-6 border border-gray-800 overflow-y-auto max-h-[300px] md:max-h-none'>

      {/* Bet amount */}
      <div className='space-y-2'>
        <label className='text-sm text-gray-300'>Bet amount</label>
        <div className='flex gap-2'>
          <Input
            type='number'
            value={gameState.betAmount}
            onChange={(e) => handleBetChange(e.target.value)}
            className='flex-1 bg-gray-800 text-white border-gray-700 h-10 md:h-9'
          />
          <Button
            variant='default'
            onClick={() => handleBetChange(String(gameState.betAmount / 2))}
            className='text-white border-gray-700 hover:bg-gray-700 px-2 h-10 md:h-9 text-sm'>
            x1/2
          </Button>
          <Button
            variant='default'
            onClick={() => handleBetChange(String(gameState.betAmount * 2))}
            className='text-white border-gray-700 hover:bg-gray-700 px-2 h-10 md:h-9 text-sm'>
            x2
          </Button>
        </div>
      </div>

      {/* Send ball */}
      <Button
        className='w-full sticky bottom-2 bg-purple-600 hover:bg-purple-700 text-white py-3 h-auto text-base'
        onClick={onSendBall}>
        Play
      </Button>

      {/* Sound switch */}
      <div className='flex items-center justify-between text-white'>
        <div className='flex items-center gap-2'>
          {gameState.sound ? <Volume2 size={20} /> : <VolumeX size={20} />}
        </div>
        <Switch
          checked={gameState.sound}
          onCheckedChange={(checked) => updateGameState({ sound: checked })}
        />
      </div>
    </div>
  );
}
