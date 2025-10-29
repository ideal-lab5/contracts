"use client"

import { useRef, useState } from "react";
import { ControlPanel } from "@/components/control-panel";
import { GameBoard, type GameBoardHandle } from "@/components/game-board";
import { useGameContext } from "@/context/GameContext";
import { usePolkadot } from "@/context/PolkadotContext";
import { BN, BN_ONE, u8aToBn, u8aToNumber, u8aToString } from "@polkadot/util";

export default function Plinko() {

  const { gameState, updateGameState, isReady } = useGameContext();

  const { alice, idncClient, contract, loading} = usePolkadot();

  const [isWaitingForResult, setIsWaitingForResult] = useState(false);
  const [transactionStatus, setTransactionStatus] = useState("");

  const gameBoardRef = useRef<GameBoardHandle>(null);

  const handleBallEnd = (multiplier: number) => {
    const winAmount = gameState.betAmount * multiplier;
    // This should be updated with the Wallet's balance
    updateGameState({
      balance: gameState.balance - gameState.betAmount + winAmount,
      isRunning: false,
    });

    setIsWaitingForResult(false);
    setTransactionStatus("");

    if (gameState.sound) {
      const audio = new Audio("/drop-sound.mp3");
      audio.play();
    }
  };

  const handleSendBall = async () => {
    if (!alice || !idncClient || !contract) return;
    if (gameState.balance < gameState.betAmount) return;
    if (isWaitingForResult) return;

    setIsWaitingForResult(true);
    setTransactionStatus("Preparing transaction...");
      // Prepare the bet object
    const bet = {
      bet: gameState.betAmount, // Adjust based on your Bet struct
    };

    // Value to send with payable message (in smallest unit)
    const value = 1000000000000; // 1 token with 12 decimals

    setTransactionStatus("Estimating gas...");

    // Estimate gas
    const { gasRequired } = await contract.query.joinGame(
      alice.address,
      {
        gasLimit: idncClient.registry.createType('WeightV2', {
          refTime: 1000000000000,
          proofSize: 100000,
        }) as any,
        value,
      },
      bet
    ) as any;

    setTransactionStatus("Waiting for signature...");

    const tx = contract.tx.joinGame({gasLimit: gasRequired, value: gameState.betAmount}, gameState.betAmount)
    tx.signAndSend(alice, (result) => {})

    const unsubscribe: any = await idncClient.query.system.events((events: any[]) => {
      events.forEach((record) => {
        const event: any  = record.event
        if (event.section === 'contracts' && event.method === 'ContractEmitted') {
          const contractId = event.data.contract.toString();
          if (contractId.toString() === contract.address.toString()) {
            const decoded = contract.abi.decodeEvent(record);
            if (decoded.event.identifier === 'plinko_contract::contract::GamePlayed') {
                const results: any[] = decoded.args[0] as unknown as any[];
                const path: any = decoded.args[1];
                console.log("results: ", results)
                results.forEach((result: any) => {
                  const [accountId, winnings] = result
                  let alice_translated_id = "15oF4uVJwmo4TdGW7VfQxNLavjCXviqxT9S1MgbjMNHr6Sp5"
                  console.log("accountId: ", accountId.toString())
                  console.log("Alice's accountId: ", alice.address)
                  console.log("winnings: ", winnings)
                  if(accountId.toString() == alice_translated_id) {
                    console.log("account id: ", accountId.toString())
                    console.log("Winnings: ", winnings.toNumber())
                    console.log("Alice's ball is being sent")
                    console.log("path: ", path)
                    setIsWaitingForResult(false);
                    setTransactionStatus("");
                    gameBoardRef.current?.dropBall(path)
                    unsubscribe()
                  }
                })
            }
          }
        }
      });
    });   
  };

  // Handle loading state in the page
  if (!isReady || loading) {
    return (
      <div className="min-h-screen bg-[#13141a] text-white flex items-center justify-center">
        <div className="text-center">
          <div className="text-xl mb-2">Loading game...</div>
          <div className="text-sm text-gray-400">Connecting to blockchain</div>
        </div>
      </div>
    );
  }

  return (
    <div className='min-h-screen bg-[#13141a] text-white'>
      <div className='max-w-6xl mx-auto p-4 md:p-8 flex flex-col h-[calc(100vh-60px)]'>
        {/* Header */}
        <header className='flex flex-col md:flex-row justify-between items-center gap-4 mb-4'>
          <div className='flex items-center gap-4 text-2xl order-1 md:order-none'>
            <span>$</span>
            <span className='font-mono'>{gameState.balance.toFixed(2)}</span>
          </div>
        </header>

        {/* Transaction Status Overlay */}
        {isWaitingForResult && (
          <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <div className="bg-[#1a1b26] p-8 rounded-lg shadow-xl border border-gray-700 max-w-md">
              <div className="flex flex-col items-center gap-4">
                {/* Spinner */}
                <div className="w-16 h-16 border-4 border-gray-600 border-t-blue-500 rounded-full animate-spin"></div>
                
                {/* Status Text */}
                <div className="text-center">
                  <h3 className="text-xl font-bold mb-2">Processing Game</h3>
                  <p className="text-gray-400">{transactionStatus}</p>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Main Content */}
        <div className='flex flex-col md:flex-row gap-4 flex-grow overflow-hidden'>
          <div className='flex-none w-full md:w-72 order-2 md:order-1'>
            <ControlPanel
              onSendBall={handleSendBall}
            />
          </div>
          <div className='flex-grow order-1 md:order-2 h-[calc(100vh-200px)] md:h-auto'>
            {/* Render GameBoard only after isMobile is determined */}
            <GameBoard
              ref={gameBoardRef}
              risk={gameState.risk}
              rows={gameState.rows}
              onBallEnd={handleBallEnd}
              isMobile={false}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
