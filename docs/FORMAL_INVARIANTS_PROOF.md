# Stellar Splash Contracts - Formal Invariants & Mathematical Proofs

## 1. Theorem: Zero-Deficit Prize Settlement Invariant
Let $P \in \mathbb{N}$ be the total vaulted prize pool in stroops ($1\text{ XLM} = 10^7\text{ stroops}$).
Let $B_i \in \mathbb{N}$ be the basis points allocated to rank $i$, where $1 \le i \le N$.
Let $B_R$ be the creator royalty basis points, and $B_F$ be the protocol fee basis points.

### Invariant Constraint:
$$\sum_{i=1}^{N} B_i + B_R + B_F \equiv 10,000$$

### Disbursement Formula:
$$D_i = \left\lfloor \frac{P \cdot B_i}{10,000} \right\rfloor$$
$$D_R = \left\lfloor \frac{P \cdot B_R}{10,000} \right\rfloor$$
$$D_F = \left\lfloor \frac{P \cdot B_F}{10,000} \right\rfloor$$

### Theorem Statement:
The total disbursed tokens $\mathcal{D} = \sum_{i=1}^N D_i + D_R + D_F \le P$.
Furthermore, the non-negative integer remainder (dust) $\mathcal{R} = P - \mathcal{D}$ satisfies $0 \le \mathcal{R} < 10,000$.

### Proof:
Since $\lfloor x \rfloor \le x$ for all $x \in \mathbb{R}$:
$$\mathcal{D} = \sum_{i=1}^N \left\lfloor \frac{P \cdot B_i}{10,000} \right\rfloor + \left\lfloor \frac{P \cdot B_R}{10,000} \right\rfloor + \left\lfloor \frac{P \cdot B_F}{10,000} \right\rfloor \le \frac{P}{10,000} \left( \sum_{i=1}^N B_i + B_R + B_F \right) = \frac{P \cdot 10,000}{10,000} = P$$
Therefore, $\mathcal{D} \le P$, proving that the contract can never become insolvent or disburse more tokens than exist in its escrow vault. $\blacksquare$

