import React, { useState, useEffect } from 'react';
import { ApiPromise, WsProvider } from '@polkadot/api';
import { ContractPromise } from '@polkadot/api-contract';
import { Keyring } from '@polkadot/keyring';
import { cryptoWaitReady } from '@polkadot/util-crypto';
import contractMetadata from './gacha.json';
import './App.css';
import { BN, BN_ONE, u8aToBn, u8aToNumber, u8aToString } from "@polkadot/util";

// Simple trait visualization using CSS and emojis
const TraitVisualizer = ({ traits = [] }) => {
  const getTraitDisplay = (traitId) => {
    // Simple mapping of trait IDs to visual elements
    const traitMap = {
      // Common traits (1-20) - Basic colors/shapes
      1: { emoji: '🔴', name: 'Red Circle', rarity: 'Common' },
      2: { emoji: '🟦', name: 'Blue Square', rarity: 'Common' },
      3: { emoji: '🟢', name: 'Green Circle', rarity: 'Common' },
      4: { emoji: '🟡', name: 'Yellow Circle', rarity: 'Common' },
      5: { emoji: '🟣', name: 'Purple Circle', rarity: 'Common' },
      6: { emoji: '🟠', name: 'Orange Circle', rarity: 'Common' },
      7: { emoji: '⚫', name: 'Black Circle', rarity: 'Common' },
      8: { emoji: '⚪', name: 'White Circle', rarity: 'Common' },
      9: { emoji: '🔶', name: 'Orange Diamond', rarity: 'Common' },
      10: { emoji: '🔷', name: 'Blue Diamond', rarity: 'Common' },
      11: { emoji: '🔸', name: 'Small Orange Diamond', rarity: 'Common' },
      12: { emoji: '🔹', name: 'Small Blue Diamond', rarity: 'Common' },
      13: { emoji: '▫️', name: 'White Square', rarity: 'Common' },
      14: { emoji: '▪️', name: 'Black Square', rarity: 'Common' },
      15: { emoji: '🔺', name: 'Red Triangle', rarity: 'Common' },
      16: { emoji: '🔻', name: 'Red Triangle Down', rarity: 'Common' },
      17: { emoji: '💠', name: 'Diamond Shape', rarity: 'Common' },
      18: { emoji: '🔘', name: 'Radio Button', rarity: 'Common' },
      19: { emoji: '🔳', name: 'White Square Button', rarity: 'Common' },
      20: { emoji: '🔲', name: 'Black Square Button', rarity: 'Common' },

      // Rare traits (21-40) - Animals/faces
      21: { emoji: '🐱', name: 'Cat Face', rarity: 'Rare' },
      22: { emoji: '🐶', name: 'Dog Face', rarity: 'Rare' },
      23: { emoji: '🐸', name: 'Frog Face', rarity: 'Rare' },
      24: { emoji: '🦊', name: 'Fox Face', rarity: 'Rare' },
      25: { emoji: '🐻', name: 'Bear Face', rarity: 'Rare' },
      26: { emoji: '🐼', name: 'Panda Face', rarity: 'Rare' },
      27: { emoji: '🦁', name: 'Lion Face', rarity: 'Rare' },
      28: { emoji: '🐯', name: 'Tiger Face', rarity: 'Rare' },
      29: { emoji: '🐨', name: 'Koala Face', rarity: 'Rare' },
      30: { emoji: '🐷', name: 'Pig Face', rarity: 'Rare' },
      31: { emoji: '🐮', name: 'Cow Face', rarity: 'Rare' },
      32: { emoji: '🐵', name: 'Monkey Face', rarity: 'Rare' },
      33: { emoji: '🦄', name: 'Unicorn Face', rarity: 'Rare' },
      34: { emoji: '🐲', name: 'Dragon Face', rarity: 'Rare' },
      35: { emoji: '🦉', name: 'Owl', rarity: 'Rare' },
      36: { emoji: '🦅', name: 'Eagle', rarity: 'Rare' },
      37: { emoji: '🐧', name: 'Penguin', rarity: 'Rare' },
      38: { emoji: '🦋', name: 'Butterfly', rarity: 'Rare' },
      39: { emoji: '🐝', name: 'Bee', rarity: 'Rare' },
      40: { emoji: '🦜', name: 'Parrot', rarity: 'Rare' },

      // Epic traits (41-50) - Special symbols
      41: { emoji: '⭐', name: 'Star', rarity: 'Epic' },
      42: { emoji: '💎', name: 'Diamond', rarity: 'Epic' },
      43: { emoji: '👑', name: 'Crown', rarity: 'Epic' },
      44: { emoji: '🏆', name: 'Trophy', rarity: 'Epic' },
      45: { emoji: '🔥', name: 'Fire', rarity: 'Epic' },
      46: { emoji: '⚡', name: 'Lightning', rarity: 'Epic' },
      47: { emoji: '🌟', name: 'Glowing Star', rarity: 'Epic' },
      48: { emoji: '💫', name: 'Dizzy Star', rarity: 'Epic' },
      49: { emoji: '✨', name: 'Sparkles', rarity: 'Epic' },
      50: { emoji: '🎭', name: 'Drama Masks', rarity: 'Epic' },
    };

    return traitMap[traitId] || { emoji: '❓', name: `Unknown #${traitId}`, rarity: 'Unknown' };
  };

  const getRarityColor = (rarity) => {
    switch (rarity) {
      case 'Common': return '#94a3b8';
      case 'Rare': return '#3b82f6';
      case 'Epic': return '#a855f7';
      default: return '#6b7280';
    }
  };

  return (
    <div className="bg-gradient-to-br from-gray-100 to-gray-200 p-6 rounded-lg border-2 border-gray-300 min-h-[200px]">
      <h3 className="text-lg font-bold mb-4 text-center">Your NFT</h3>
      <div className="grid grid-cols-3 gap-2 mb-4">
        {traits.map((traitId, index) => {
          const trait = getTraitDisplay(traitId);
          return (
            <div
              key={index}
              className="flex flex-col items-center p-2 bg-white rounded-lg shadow-sm border"
              style={{ borderColor: getRarityColor(trait.rarity) }}
            >
              <div className="text-2xl mb-1">{trait.emoji}</div>
              <div className="text-xs text-center">
                <div className="font-medium">{trait.name}</div>
                <div
                  className="text-xs font-bold"
                  style={{ color: getRarityColor(trait.rarity) }}
                >
                  {trait.rarity}
                </div>
              </div>
            </div>
          );
        })}
      </div>
      <div className="text-center text-sm text-gray-600">
        Total Traits: {traits.length}
      </div>
    </div>
  );
};

const MAX_CALL_WEIGHT = new BN(1_000_000_000_000).isub(BN_ONE);
const PROOFSIZE = new BN(100_000);

const GachaNFTApp = () => {
  const [api, setApi] = useState(null);
  const [alice, setAlice] = useState(null);
  const [contract, setContract] = useState(null);
  const [ownedNFTs, setOwnedNFTs] = useState([]);
  const [selectedNFT, setSelectedNFT] = useState(null);
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState('Ready to connect');
  const [prices, setPrices] = useState({ mint: 100_000_000_000, spin: 10_000_000_000 });

  // Contract address
  const CONTRACT_ADDRESS = '5HJxWYVEoJqNhKJ9Pp3MEjzvUm8YJqCgh8d1a5bw7iKKQhUo';

  useEffect(() => {
    connectWallet();
  }, []);

  const connectWallet = async () => {
    setLoading(true);
    setStatus('Connecting...');

    try {
      // This will be replaced with actual PolkadotJS imports in your real project
      // For demo purposes, we'll simulate the connection

      // In your real implementation, uncomment these lines:
      await cryptoWaitReady();
      const wsProvider = new WsProvider('ws://127.0.0.1:9933');
      const api = await ApiPromise.create({ provider: wsProvider });

      console.log('api is ready')

      const keyring = new Keyring({ type: 'sr25519' });
      const alice = keyring.addFromUri('//Alice', { name: 'Alice' });

      // Create contract instance
      const contract = new ContractPromise(api, contractMetadata, CONTRACT_ADDRESS);

      setApi(api);
      setAlice(alice);
      setContract(contract);

      // Simulate for demo
      setTimeout(async () => {
        setStatus('Connected as Alice');
        setLoading(false);

        // Load initial data
        await Promise.all([
          loadPrices(api, contract, alice),
          loadOwnedNFTs(api, contract, alice)
        ]);
      }, 1000);

    } catch (error) {
      console.error('Connection failed:', error);
      setStatus('Connection failed');
      setLoading(false);
    }
  };

  const loadPrices = async (api, contract, alice) => {
    try {
      const { result } = await contract.query.getPrices(alice.address, {
        gasLimit: api.createType('WeightV2', {
          refTime: MAX_CALL_WEIGHT,
          proofSize: PROOFSIZE,
        }),
        storageDepositLimit: null,
      });

      let raw = result.toHuman().Ok.data;
      const bytes = api.createType('(Bytes)', raw).toU8a().slice(3);
      const mintPrice = u8aToBn(bytes.slice(0, 16));
      const spinPrice = u8aToBn(bytes.slice(16));

      setPrices({
        mint: mintPrice,
        spin: spinPrice
      });
    } catch (error) {
      console.error('Failed to load prices:', error);
    }
  };

  const loadOwnedNFTs = async (api, contract, alice) => {
    try {
      // Real implementation:
      const { result } = await contract.query.getOwnedNfts(alice.address, {
        gasLimit: api.createType('WeightV2', {
          refTime: MAX_CALL_WEIGHT,
          proofSize: PROOFSIZE,
        }),
        storageDepositLimit: null,
      }, alice.address);
      const nftIds = result.toHuman();
      console.log(result)

      // // Load traits for each NFT
      // const nftsWithTraits = await Promise.all(
      //   nftIds.map(async (id) => {
      //     const { result } = await contract.query.getNftTraits(alice.address, { gasLimit: -1 }, id);
      //     return { id: parseInt(id), traits: result.toHuman() || [] };
      //   })
      // );'

      const nftsWithTraits = [];

      // // Simulate for demo
      // const mockNFTs = [
      //   { id: 1, traits: [1, 25, 41] },
      //   { id: 2, traits: [5, 12, 28, 45] }
      // ];

      setOwnedNFTs(nftsWithTraits);
      if (nftsWithTraits.length > 0) {
        setSelectedNFT(nftsWithTraits[0]);
      }
    } catch (error) {
      console.error('Failed to load NFTs:', error);
    }
  };

  const mintNFT = async () => {
    if (!api || !alice || !contract) {
      console.log('Not connected yet, simulating mint...');
      // Simulate mint for demo
      const newNFT = {
        id: ownedNFTs.length + 1,
        traits: [Math.floor(Math.random() * 20) + 1, Math.floor(Math.random() * 20) + 1, Math.floor(Math.random() * 20) + 1]
      };

      setOwnedNFTs(prev => [...prev, newNFT]);
      setSelectedNFT(newNFT);
      setStatus('NFT Minted! (simulated)');
      return;
    }

    setLoading(true);
    setStatus('Minting NFT...');

    try {
      await contract.tx.mintNft({
        gasLimit: api.createType('WeightV2', {
          refTime: MAX_CALL_WEIGHT,
          proofSize: PROOFSIZE,
        }),
        storageDepositLimit: null,
        value: prices.value, 
      },
    ).signAndSend(alice, (result) => {
        if (result.status.isInBlock) {
          console.log('Transaction included in block');
        } else if (result.status.isFinalized) {
          console.log('Transaction finalized');
        } else if (result.isError) {
          console.err('uh oh  ' + result)
        }
      });

      // Reload NFTs after successful mint
      await loadOwnedNFTs(api, contract, alice);

      // Simulate for demo
      setTimeout(() => {
        const newNFT = {
          id: ownedNFTs.length + 1,
          traits: [Math.floor(Math.random() * 20) + 1, Math.floor(Math.random() * 20) + 1, Math.floor(Math.random() * 20) + 1]
        };

        setOwnedNFTs(prev => [...prev, newNFT]);
        setSelectedNFT(newNFT);
        setStatus('NFT Minted!');
        setLoading(false);
      }, 2000);

    } catch (error) {
      console.error('Minting failed:', error);
      setStatus('Minting failed');
      setLoading(false);
    }
  };

  const spinGacha = async () => {
    if (!selectedNFT) return;

    if (!api || !alice || !contract) {
      console.log('Not connected yet, simulating spin...');
      // Simulate spin for demo
      const newTraitId = Math.floor(Math.random() * 50) + 1;
      const updatedNFT = {
        ...selectedNFT,
        traits: [...selectedNFT.traits, newTraitId]
      };

      setOwnedNFTs(prev =>
        prev.map(nft => nft.id === selectedNFT.id ? updatedNFT : nft)
      );
      setSelectedNFT(updatedNFT);
      setStatus('New trait added! (simulated)');
      return;
    }

    setLoading(true);
    setStatus('Spinning gacha...');

    try {
      // Real implementation:
      const { gasRequired } = await contract.query.spinGacha(alice.address, {
        gasLimit: -1,
        value: prices.spin
      }, selectedNFT.id);

      const tx = contract.tx.spinGacha({
        gasLimit: gasRequired,
        value: prices.spin
      }, selectedNFT.id);

      await new Promise((resolve, reject) => {
        tx.signAndSend(alice, (result) => {
          if (result.status.isInBlock) {
            console.log('Transaction included in block');
          } else if (result.status.isFinalized) {
            console.log('Transaction finalized');
            resolve(result);
          } else if (result.isError) {
            reject(new Error('Transaction failed'));
          }
        });
      });

      // Reload NFT traits after successful spin
      await loadOwnedNFTs();

      // Simulate for demo
      setTimeout(() => {
        const newTraitId = Math.floor(Math.random() * 50) + 1;
        const updatedNFT = {
          ...selectedNFT,
          traits: [...selectedNFT.traits, newTraitId]
        };

        setOwnedNFTs(prev =>
          prev.map(nft => nft.id === selectedNFT.id ? updatedNFT : nft)
        );
        setSelectedNFT(updatedNFT);
        setStatus('New trait added!');
        setLoading(false);
      }, 1500);

    } catch (error) {
      console.error('Gacha spin failed:', error);
      setStatus('Spin failed');
      setLoading(false);
    }
  };

  return (
    <div className="gacha-app">
      <div className="gacha-container">
        <h1 className="gacha-title">Gacha NFT Collector</h1>
        <p className="gacha-subtitle">Collect random traits for your unique NFTs!</p>

        <div className="gacha-card">
          <div className="status-bar">
            <div className="status-info">
              <h2>Status: {status}</h2>
              <p>
                Contract: {CONTRACT_ADDRESS.slice(0, 8)}...{CONTRACT_ADDRESS.slice(-8)}
              </p>
              <p>
                Mint: {(prices.mint / 1000000000000).toFixed(1)} tokens |
                Spin: {(prices.spin / 1000000000000).toFixed(1)} tokens
              </p>
            </div>
            <div className="button-group">
              <button
                onClick={mintNFT}
                disabled={loading}
                className="gacha-button button-mint"
              >
                {loading ? 'Minting...' : 'Mint NFT'}
              </button>
              <button
                onClick={spinGacha}
                disabled={loading || !selectedNFT}
                className="gacha-button button-spin"
              >
                {loading ? 'Spinning...' : 'Spin Gacha'}
              </button>
            </div>
          </div>
        </div>

        <div className="main-grid">
          {/* NFT List */}
          <div className="nft-list">
            <h3>Your NFTs ({ownedNFTs.length})</h3>
            <div className="nft-items">
              {ownedNFTs.length === 0 ? (
                <p className="nft-empty">No NFTs yet. Mint your first one!</p>
              ) : (
                ownedNFTs.map(nft => (
                  <div
                    key={nft.id}
                    onClick={() => setSelectedNFT(nft)}
                    className={`nft-item ${selectedNFT?.id === nft.id ? 'selected' : ''}`}
                  >
                    <div className="nft-item-title">NFT #{nft.id}</div>
                    <div className="nft-item-subtitle">{nft.traits.length} traits</div>
                  </div>
                ))
              )}
            </div>
          </div>

          {/* NFT Visualizer */}
          <div>
            {selectedNFT ? (
              <TraitVisualizer traits={selectedNFT.traits} />
            ) : (
              <div className="nft-placeholder">
                <p>Select an NFT to view its traits</p>
              </div>
            )}
          </div>
        </div>

        {/* Collection Stats */}
        {ownedNFTs.length > 0 && (
          <div className="stats-section">
            <h3>Collection Stats</h3>
            <div className="stats-grid">
              <div className="stat-item">
                <div className="stat-number stat-blue">
                  {ownedNFTs.reduce((sum, nft) => sum + nft.traits.length, 0)}
                </div>
                <div className="stat-label">Total Traits</div>
              </div>
              <div className="stat-item">
                <div className="stat-number stat-green">{ownedNFTs.length}</div>
                <div className="stat-label">NFTs Owned</div>
              </div>
              <div className="stat-item">
                <div className="stat-number stat-purple">
                  {Math.max(...ownedNFTs.map(nft => nft.traits.length))}
                </div>
                <div className="stat-label">Max Traits</div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default GachaNFTApp;