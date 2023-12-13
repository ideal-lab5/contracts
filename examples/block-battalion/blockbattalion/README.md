# Block Defender

Block defender is a proof of concept to demonstrate how to develop p2p games on the etf network.

Block defender leverages *three*  distinct 'game event clocks' to drive the game. The clocks feed into each other and attempt to create a semi-sustainable feedback loop.

## Setup and Installation

Navigate to the root directory and run:

``` sh
npm i && npm run start
```

**from docker**
(TODO: DNE yet)
``` sh
docker run -p 3000:3000 ideallabs/bitbattalion 
```

By default, the game will connect to the ETF network testnet.

## Gameplay

The game starts with a finite [MxN] grid. The goal of the game is to increase your total power and to dominate the grid.

Each player starts with a base, a single cell in the grid. Each cell in the grid has a default power of (1), and each base has a default (and minimum) power of (2).

Some thoughts/ideas that are not implemented:
- make a base's children be ephemeral: children leak power over time, and the further away from the core the faster power is drained

### Getting Started: Creating a Base

Players can each owns a cell which serves as the core of their empire. In order to be eligible to earn resources, players must have an owned base in the game. By default, each base has a power level of 2. When players create a base for the first time, they are awarded 10 iron, a fictional in-game resource and **not** a cryptocurrency/NFT/etc. 

### Earning Resources

The in-game resource used is called iron. Iron can be mined by any player with a base (which serves as their mining operation). Iron is distributed on a regular basis through a game event clock contract powered by timelock encryption. The resource loop is a periodic game of bit roulette. Between each resource allocation event, each player can attempt to mine as many times as they'd like, which means choosing a bit, encrypting it, and sending the ciphertext to the contract. 

Each event distributes a maximum of 100 iron across all players. The rules of this mechanism are simple. Let $P = \{P_1, ..., P_n\}$ be the players and  $e_1, e_2, ..., e_k, ...$ represent the events. Then for any event $e_k$, each player $P_i$ choose some $m_i \in \{0, 1\}$ and encrypts the message for the event $e_k$ with timelock encryption. When each player's ciphertext is decrypted, the winners are the set of players $W = \{P_i \in P: m_i = \oplus_{P_j \in P} m_j\}$. Thus, each round distributed $100/|W|$ iron to each member of the set $W$.

### Expanding a Base and Conquering Neighboring Cells

Neighboring cells can be conquered under certain conditions. To be precise, for a neighbor cell N and and owned cell C, it must hold that $C.powerLevel - N.powerLevel \geq 4$. Or, to be more general, $C.powerLevel - N.powerLevel \geq 2*OWNED\;CELL\;MIN$.

When a neighbor is conquered, the conquerer's cell splits its power to create a new owned cell. First it 'spends' power levels to conquer the neighbor, and then it must create a new cell with a power level of at least (2). To elaborate, consider the following example:

> A base has power level (7) and it's unoccupied neighbor has power level (1). To conquer the cell, first (1) power level is spent to conquer the neighbor (w/ power level (1)), and then the base's power is split to create a new owned cell with power level (2). So at the end, the player has a base with power level (4), and a child with power level (2), ultimately reducing power the total power level of the empire. 

### Increasing Power Levels

Players spend iron to increase their power level. The cost in iron to enhance a base follows an exponential curve, given by: $f(x) = \frac{\frac{x^2}{2} + 2}{2} = \frac{1}{4}x^2 + \frac{1}{2}x$. Each time a base's power is doubled the cost in iron also doubles. For example, it follows the table:

|start|end|cost|
|--|--|--|
|2|4|2|
|4|6|4|
|6|8|8|
|8|10|16|
|10|12|32|

## Fast Forward

Each game event clock can be conditionally fast-forwarded to the latest valid round number. This is possible only when, given consecutive round numbers $r_1, ..., r_k$, that there are:
1) blocks authored in each slot
2) no timelocked messages issued for the events

In such a case, we can fast-forward the event clock's round number to the latest round such that there is no block in the slot.