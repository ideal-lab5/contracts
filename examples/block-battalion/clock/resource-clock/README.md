# The Resource Game Event Clock

This is the event clock for "mining" events in block defender. 
It is a modified version of bit roulette where players win a round if the bit they issued matches the parity of the sum of all bits played.

## Round win conditions

That is, for a player who chose a bit $b \in \{0, 1\}^*$, the player wins iff $b = \sum_j b_j \mod{2}$.

## Advance_clock

When advancing the clock, the event allocates resources (iron) to winners. It distributes a total of 100 iron per round. If $W$ is the set of all winners, then each winner gets $100 / |W|$ iron. Using floats, this isn't always going to be perfect but we are ok with that for the sake of simplicity.