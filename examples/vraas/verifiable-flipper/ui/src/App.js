import React, { useState, useEffect } from 'react';
import { ApiPromise, WsProvider } from '@polkadot/api';
import { ContractPromise } from '@polkadot/api-contract';
import { Keyring } from '@polkadot/keyring';
import { cryptoWaitReady } from '@polkadot/util-crypto';
import contractMetadata from './verifiable_flipper.json';
import './App.css';
import { BN, BN_ONE, u8aToBn } from "@polkadot/util";

const MAX_CALL_WEIGHT = new BN(1_000_000_000_000).isub(BN_ONE);
const PROOFSIZE = new BN(500_000);

// Coin flip animation component
const CoinFlip = ({ result, isFlipping }) => {
  return (
    <div className={`coin-container ${isFlipping ? 'flipping' : ''}`}>
      <div className={`coin ${result === true ? 'heads' : 'tails'}`}>
        <div className="coin-face heads-face">
          <span className="coin-emoji">👑</span>
          <div className="coin-label">HEADS</div>
        </div>
        <div className="coin-face tails-face">
          <span className="coin-emoji">🪙</span>
          <div className="coin-label">TAILS</div>
        </div>
      </div>
    </div>
  );
};

// All Rounds Display Component
const AllRoundsDisplay = ({ rounds, onSelectRound, selectedRound }) => {
  if (!rounds || rounds.length === 0) {
    return (
      <div className="rounds-container">
        <h3 className="rounds-title">📊 All Rounds</h3>
        <div className="rounds-empty">
          <p>No rounds completed yet. Place some bets and wait for randomness!</p>
        </div>
      </div>
    );
  }

  return (
    <div className="rounds-container">
      <h3 className="rounds-title">📊 All Rounds (0 → {rounds[rounds.length - 1].roundNumber})</h3>
      <div className="rounds-list">
        {rounds.map((round) => (
          <div
            key={round.roundNumber}
            onClick={() => onSelectRound(round.roundNumber)}
            className={`round-card ${selectedRound === round.roundNumber ? 'selected' : ''}`}
          >
            <div className="round-header">
              <span className="round-number">Round {round.roundNumber}</span>
              <span className={`round-status ${round.winners.length > 0 ? 'has-winners' : 'no-winners'}`}>
                {round.winners.length > 0 ? '🏆' : '💤'}
              </span>
            </div>
            <div className="round-info">
              <div className="round-stat">
                <span className="round-stat-label">Winners:</span>
                <span className="round-stat-value">{round.winners.length}</span>
              </div>
              {round.winners.length > 0 && (
                <div className="round-winners-preview">
                  {round.winners.slice(0, 3).map((winner, idx) => (
                    <span key={idx} className="winner-badge">
                      {winner.slice(0, 6)}...
                    </span>
                  ))}
                  {round.winners.length > 3 && (
                    <span className="winner-badge-more">+{round.winners.length - 3}</span>
                  )}
                </div>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

// Flip history display component
const FlipHistory = ({ flips, onSelectRound }) => {
  return (
    <div className="flip-history">
      <h3 className="history-title">My Flip History</h3>
      <div className="history-items">
        {flips.length === 0 ? (
          <p className="history-empty">No flips yet. Place your first bet!</p>
        ) : (
          flips.map((flip, index) => (
            <div
              key={index}
              onClick={() => flip.round && onSelectRound(flip.round)}
              className={`history-item ${flip.won ? 'won' : 'lost'} ${flip.pending ? 'pending' : ''}`}
            >
              <div className="history-header">
                <span className="history-result">
                  {flip.pending ? '⏳' : flip.result === true ? '👑' : '🪙'}
                </span>
                <span className="history-outcome">
                  {flip.pending ? 'Pending' : flip.won ? 'WON' : 'LOST'}
                </span>
              </div>
              <div className="history-details">
                <div className="history-bet">Bet: {(flip.betAmount / 1_000_000_000_000).toFixed(4)} tokens</div>
                {flip.round && <div className="history-round">Round: {flip.round}</div>}
                <div className="history-call">
                  Called: {flip.callHeads ? 'Heads 👑' : 'Tails 🪙'}
                </div>
                {!flip.pending && (
                  <div className="history-actual">
                    Result: {flip.result ? 'Heads 👑' : 'Tails 🪙'}
                  </div>
                )}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
};

// Winners display component
const WinnersDisplay = ({ winners, round }) => {
  if (!winners || winners.length === 0) {
    return (
      <div className="winners-container">
        <h3 className="winners-title">🏆 Round {round}</h3>
        <div className="winners-empty">
          <p>No winners for this round</p>
        </div>
      </div>
    );
  }

  return (
    <div className="winners-container">
      <h3 className="winners-title">🏆 Round {round} Winners</h3>
      <div className="winners-list">
        {winners.map((winner, index) => (
          <div key={index} className="winner-item">
            <span className="winner-emoji">👑</span>
            <span className="winner-address">
              {winner.slice(0, 8)}...{winner.slice(-8)}
            </span>
          </div>
        ))}
      </div>
      <div className="winners-count">
        Total Winners: {winners.length}
      </div>
    </div>
  );
};

const VerifiableFlipperApp = () => {
  const [api, setApi] = useState(null);
  const [alice, setAlice] = useState(null);
  const [contract, setContract] = useState(null);
  const [flipHistory, setFlipHistory] = useState([]);
  const [allRounds, setAllRounds] = useState([]);
  const [selectedRound, setSelectedRound] = useState(null);
  const [roundWinners, setRoundWinners] = useState(null);
  const [loading, setLoading] = useState(false);
  const [loadingRounds, setLoadingRounds] = useState(false);
  const [status, setStatus] = useState('Ready to connect');
  const [betAmount, setBetAmount] = useState('10');
  const [callHeads, setCallHeads] = useState(true);
  const [subscriptionId, setSubscriptionId] = useState(null);
  const [isFlipping, setIsFlipping] = useState(false);
  const [currentFlip, setCurrentFlip] = useState(null);

  const CONTRACT_ADDRESS = '12tNihwnF856fNKnbtbu96uFRCg8idRAKDUs8ztDm1XWXxkr';

  useEffect(() => {
    connectWallet();
  }, []);

  // Auto-refresh rounds every 30 seconds
  useEffect(() => {
    if (api && contract && alice) {
      const interval = setInterval(() => {
        loadAllRounds(api, contract, alice);
      }, 30000);
      return () => clearInterval(interval);
    }
  }, [api, contract, alice]);

  const connectWallet = async () => {
    setLoading(true);
    setStatus('Connecting...');

    try {
      await cryptoWaitReady();
      // const wsProvider = new WsProvider('ws://127.0.0.1:9944');
      const wsProvider = new WsProvider('wss://idnc0-testnet.idealabs.network:443');
      const api = await ApiPromise.create({ provider: wsProvider });

      console.log('API is ready');

      const keyring = new Keyring({ type: 'sr25519' });
      const alice = keyring.addFromUri('//Alice', { name: 'Alice' });
      console.log(alice.address);

      // Create contract instance
      const contract = new ContractPromise(api, contractMetadata, CONTRACT_ADDRESS);

      setApi(api);
      setAlice(alice);
      setContract(contract);

      setTimeout(async () => {
        console.log('connected as alice')
        setStatus('Connected as Alice');
        setLoading(false);

        // Load initial data
        await Promise.all([
          loadSubscriptionId(api, contract, alice),
          loadPendingFlips(api, contract, alice),
          loadAllRounds(api, contract, alice)
        ]);
      }, 1000);

    } catch (error) {
      console.error('Connection failed:', error);
      setStatus('Connection failed');
      setLoading(false);
    }
  };

  const loadSubscriptionId = async (api, contract, alice) => {
    try {
      const { result, output } = await contract.query.getSubscriptionId(alice.address, {
        gasLimit: api.createType('WeightV2', {
          refTime: MAX_CALL_WEIGHT,
          proofSize: PROOFSIZE,
        }),
        storageDepositLimit: null,
      });

      const subId = output.toHuman().Ok;
      setSubscriptionId(subId);
      console.log('Subscription ID:', subId);
    } catch (error) {
      console.error('Failed to load subscription ID:', error);
    }
  };

  const loadPendingFlips = async (api, contract, alice) => {
    try {
      const { result, output } = await contract.query.getPendingFlips(alice.address, {
        gasLimit: api.createType('WeightV2', {
          refTime: MAX_CALL_WEIGHT,
          proofSize: PROOFSIZE,
        }),
        storageDepositLimit: null,
      });

      console.log(output.toHuman())
      const pending = output.toHuman()?.Ok || [];
      console.log('Pending flips:', pending);

      if (pending == []) {
        return ;
      }
      // Add pending flips to history
      const pendingFlips = pending.map(flip => {
        let bet = parseFloat(flip[1].replace(/,/g, ''));
        return ({
        player: flip[0],
        betAmount: bet,
        callHeads: flip[2],
        pending: true,
      })});

      setFlipHistory(prev => [...pendingFlips, ...prev.filter(f => !f.pending)]);
    } catch (error) {
      console.error('Failed to load pending flips:', error);
    }
  };

  const getCurrentRound = async (api, contract, caller) => {
    try {
      const { result, output } = await contract.query.getRound(caller, {
        gasLimit: api.createType('WeightV2', {
          refTime: MAX_CALL_WEIGHT,
          proofSize: PROOFSIZE,
        }),
        storageDepositLimit: null,
      });

      // Check if the query itself failed
      if (result.isErr) {
        console.error('Query error:', result.asErr.toHuman());
        return null;
      }


      // Get the human-readable output
      const roundValue = output.toHuman();
      return roundValue.Ok;
    } catch (error) {
      console.error('Failed to get round:', error);
      return null;
    }
  };

  const loadAllRounds = async (api, contract, alice) => {
    setLoadingRounds(true);
    console.log('Loading all rounds...');

    try {
      const rounds = [];
      let roundNumber = 0;
      let consecutiveEmpty = 0;
      const maxConsecutiveEmpty = 5; // Stop after 5 consecutive empty rounds

      // get the latest round
      const currentRound = await getCurrentRound(api, contract, alice.address);
      if (currentRound !== null) {
        console.log('Current round is:', currentRound);
        // Use currentRound as the max for loadAllRounds
      } else {
        console.log('No round data available');
      }

      // Query rounds until we hit several empty ones in a row
      while (roundNumber <= currentRound) {
        try {
          const { result, output } = await contract.query.getWinners(alice.address, {
            gasLimit: api.createType('WeightV2', {
              refTime: MAX_CALL_WEIGHT,
              proofSize: PROOFSIZE,
            }),
            storageDepositLimit: null,
          }, roundNumber);

          const winners = output.toHuman()?.Ok || null;

          if (winners != null) {
            rounds.push({
              roundNumber,
              winners: Array.isArray(winners) ? winners : [],
            });
          }

          roundNumber++;
        } catch (error) {
          console.error(`Error loading round ${roundNumber}:`, error);
          break;
        }
      }

      console.log(`Loaded ${rounds.length} rounds (checked up to round ${roundNumber - 1})`);
      setAllRounds(rounds);
    } catch (error) {
      console.error('Failed to load all rounds:', error);
    } finally {
      setLoadingRounds(false);
    }
  };

  const loadWinnersForRound = async (round) => {
    if (!api || !contract || !alice) return;

    try {
      const { result } = await contract.query.getWinners(alice.address, {
        gasLimit: api.createType('WeightV2', {
          refTime: MAX_CALL_WEIGHT,
          proofSize: PROOFSIZE,
        }),
        storageDepositLimit: null,
      }, round);

      const winners = result.toHuman()?.Ok || [];
      setRoundWinners(winners);
      setSelectedRound(round);
      console.log(`Winners for round ${round}:`, winners);
    } catch (error) {
      console.error('Failed to load winners:', error);
    }
  };

  const createSubscription = async () => {
    if (!api || !alice || !contract) return;

    setLoading(true);
    setStatus('Creating subscription...');

    try {
      await contract.tx.createSubscription(
        {
          gasLimit: api.createType('WeightV2', {
            refTime: MAX_CALL_WEIGHT,
            proofSize: PROOFSIZE,
          }),
          storageDepositLimit: null,
          value: new BN('1000000000000'),
        },
      ).signAndSend(alice, (result) => {
        if (result.status.isInBlock) {
          console.log('Subscription transaction in block');
        } else if (result.status.isFinalized) {
          console.log('Subscription created');
          setStatus('Subscription created!');
          loadSubscriptionId(api, contract, alice);
          setLoading(false);
        } else if (result.isError) {
          console.error('Transaction error:', result);
          setStatus('Subscription failed');
          setLoading(false);
        }
      });
    } catch (error) {
      console.error('Create subscription failed:', error);
      setStatus('Subscription failed');
      setLoading(false);
    }
  };

  const placeBet = async () => {
    if (!api || !alice || !contract) {
      console.log('Not connected, simulating...');
      return;
    }

    const betValue = new BN(parseFloat(betAmount) * 1_000_000_000_000);

    setLoading(true);
    setStatus('Placing bet...');

    try {
      await contract.tx.flip(
        {
          gasLimit: api.createType('WeightV2', {
            refTime: MAX_CALL_WEIGHT,
            proofSize: PROOFSIZE,
          }),
          storageDepositLimit: null,
          value: betValue,
        },
        callHeads
      ).signAndSend(alice, (result) => {
        if (result.status.isInBlock) {
          console.log('Flip transaction in block');
        } else if (result.status.isFinalized) {
          console.log('Flip placed successfully');
          setStatus('Bet placed! Waiting for randomness...');
          setLoading(false);

          // Add pending flip to history
          const pendingFlip = {
            betAmount: betValue,
            callHeads,
            pending: true,
          };
          setFlipHistory(prev => [pendingFlip, ...prev]);

          // Poll for results and reload rounds
          setTimeout(() => {
            loadPendingFlips(api, contract, alice);
            loadAllRounds(api, contract, alice);
          }, 5000);
        } else if (result.isError) {
          console.error('Transaction error:', result);
          setStatus('Bet failed');
          setLoading(false);
        }
      });

      // Trigger flip animation
      setIsFlipping(true);
      setTimeout(() => setIsFlipping(false), 2000);

    } catch (error) {
      console.error('Bet placement failed:', error);
      setStatus('Bet failed');
      setLoading(false);
    }
  };

  // Calculate stats
  const stats = {
    totalFlips: flipHistory.filter(f => !f.pending).length,
    wins: flipHistory.filter(f => !f.pending && f.won).length,
    losses: flipHistory.filter(f => !f.pending && !f.won).length,
    totalWagered: flipHistory.reduce((sum, f) => sum + (f.betAmount || 0), 0),
    totalWon: flipHistory.filter(f => !f.pending && f.won).reduce((sum, f) => sum + (f.betAmount || 0) * 2, 0),
  };

  const winRate = stats.totalFlips > 0 ? ((stats.wins / stats.totalFlips) * 100).toFixed(1) : '0.0';

  return (
    <div className="flipper-app">
      <div className="flipper-container">
        <h1 className="flipper-title">🪙 Verifiable Coin Flipper</h1>
        <p className="flipper-subtitle">Provably fair coin flips powered by IDN randomness</p>

        <div className="flipper-card">
          <div className="status-bar">
            <div className="status-info">
              <h2>Status: {status}</h2>
              <p>
                Contract: {CONTRACT_ADDRESS.slice(0, 8)}...{CONTRACT_ADDRESS.slice(-8)}
              </p>
              <p>
                Subscription: {subscriptionId ? '✅ Active' : '❌ Not Created'}
                {loadingRounds && ' | 🔄 Loading rounds...'}
              </p>
            </div>
            <div className="button-group">
              {!subscriptionId && (
                <button
                  onClick={createSubscription}
                  disabled={loading}
                  className="flipper-button button-subscription"
                >
                  Create Subscription
                </button>
              )}
              <button
                onClick={() => loadAllRounds(api, contract, alice)}
                disabled={loadingRounds || !api}
                className="flipper-button button-refresh"
              >
                {loadingRounds ? '🔄 Loading...' : '🔄 Refresh Rounds'}
              </button>
            </div>
          </div>
        </div>

        {/* Coin Flip Area */}
        <div className="flip-section">
          <CoinFlip result={currentFlip?.result} isFlipping={isFlipping} />

          <div className="bet-controls">
            <div className="bet-input-group">
              <label htmlFor="betAmount">Bet Amount (tokens)</label>
              <input
                id="betAmount"
                type="number"
                min="0.1"
                step="0.1"
                value={betAmount}
                onChange={(e) => setBetAmount(e.target.value)}
                className="bet-input"
                disabled={loading}
              />
            </div>

            <div className="choice-toggle">
              <button
                onClick={() => setCallHeads(true)}
                className={`choice-button ${callHeads ? 'active heads' : ''}`}
                disabled={loading}
              >
                👑 HEADS
              </button>
              <button
                onClick={() => setCallHeads(false)}
                className={`choice-button ${!callHeads ? 'active tails' : ''}`}
                disabled={loading}
              >
                🪙 TAILS
              </button>
            </div>

            <button
              onClick={placeBet}
              disabled={loading || !subscriptionId}
              className="flipper-button button-flip"
            >
              {loading ? 'Flipping...' : `Flip for ${betAmount} tokens`}
            </button>
          </div>
        </div>

        {/* All Rounds Display */}
        <AllRoundsDisplay
          rounds={allRounds}
          onSelectRound={loadWinnersForRound}
          selectedRound={selectedRound}
        />

        <div className="main-grid">
          {/* Flip History */}
          <FlipHistory
            flips={flipHistory}
            onSelectRound={loadWinnersForRound}
          />

          {/* Winners Display */}
          <div>
            {selectedRound !== null ? (
              <WinnersDisplay winners={roundWinners} round={selectedRound} />
            ) : (
              <div className="winners-placeholder">
                <p>Click on a round to view winners</p>
              </div>
            )}
          </div>
        </div>

        {/* Stats Section */}
        {stats.totalFlips > 0 && (
          <div className="stats-section">
            <h3>Your Stats</h3>
            <div className="stats-grid">
              <div className="stat-item">
                <div className="stat-number stat-blue">{stats.totalFlips}</div>
                <div className="stat-label">Total Flips</div>
              </div>
              <div className="stat-item">
                <div className="stat-number stat-green">{stats.wins}</div>
                <div className="stat-label">Wins</div>
              </div>
              <div className="stat-item">
                <div className="stat-number stat-red">{stats.losses}</div>
                <div className="stat-label">Losses</div>
              </div>
              <div className="stat-item">
                <div className="stat-number stat-purple">{winRate}%</div>
                <div className="stat-label">Win Rate</div>
              </div>
              <div className="stat-item">
                <div className="stat-number stat-gold">
                  {(stats.totalWagered / 1_000_000_000_000).toFixed(2)}
                </div>
                <div className="stat-label">Total Wagered</div>
              </div>
              <div className="stat-item">
                <div className="stat-number stat-green">
                  {(stats.totalWon / 1_000_000_000_000).toFixed(2)}
                </div>
                <div className="stat-label">Total Won</div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default VerifiableFlipperApp;
