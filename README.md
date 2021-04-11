# PolkaSynthetics
Platform for synthetic derivatives on Polkadot/Acala

# How it works
I will describe below the design proposal. I assume an asset AA.

# Variables
* *L*: Liquidation ratio
* *I*: Initial IM ratio.

# Beginning of first block
Store the price $P_0$ I get from **Oracle**.

# Minting
Buyer $B_i$ expresses interest in buying quantity $X_i$. $B_i$ wires $I * X_i * P_0$ in *IM*, and opens interest.
Seller $S_i$ expresses interest in selling quantity $Y_i$. $S_i$ wires $I * SY_i * P_0$ in *IM*, and opens interest.

### Storage status
$B_i$ margin balance is $I * X_i * P_0$, open interest of buying $X_i$
$S_i$ margin balance is $I * Y_i * P_0$, open interest of selling $Y_i$

# Beginning of next block
## Price update
Ask the **Oracle** the price of the asset, call it $P_1$, store $D = P_1 - P_0$. Update the margin balances by the new price, only the inventory should be impacted by this and not the open interest. Store $P_1$ as $P_0$.

### Storage status
$B_i$ margin balance is $I * X_i * P_0 + BI_i * D$, long inventory of $BI_i$ and open interest of $BO_i$
$S_i$ margin balance is $I * Y_i * P_0 - SI_i * D$, short inventory of $SI_i$ and open interest of $SO_i$

## Liquidation
Call *M* the total margin for a participant *A*, call *T* the total interest, and *B* the inventory (open interest is $T - B$).
The needed collateral for maintaining the inventory is $B * P_0 * L$, if $B * P_0 * L > M$, then liquididate the inventory as per below.
If $B * P_0 * L < M$, but $T * P_0 * L > M$ then close out part of the total interest such that:
$$
I * P_0 * T' = M \\
T' >= B
$$

If such $T'$ is possible, total interest becomes $T' = M / (I * P_0)$ and inventory remains at *B*. If no such $T'$ is possible, which would be the case if $M / (I * P_0) < B$ or $M < B * I * P_0$, then liquidate all the open interest, so total interest becomes $T' = B$, and inventory remains at *B*. This is done to make sure that if an opposing open interest comes during that block, it does not suffer from immediate liquidation.

### Liquidation of inventory
If $B * P_0 * L > M$, liquidate the full position, so total position and inventory goes to $0$, and M is returned back to the participant *A* (but only when *A* claims it). When this happens, we need to update the inventory of other participants, because we need that $\sum BI_i = \sum SI_i$. That happens once all the liquidation round has happened.

## Interest match
If $\forall i, X_i = 0$ then no interest to match. Otherwise, call $R = \frac{\sum_i Y_i}{\sum_i X_i}$. $B_i$ has bought $min(X_i, X_i * R)$. $S_i$ has sold $min(Y_i, Y_i / R)$.

### Storage status
$B_i$ margin balance is *M*, long inventory of $BI_i = min(X_i, X_i * R)$ and open interest of $BO_i = X_i - min(X_i, X_i * R)$
$S_i$ margin balance is *M**, short inventory of $SI_i = min(Y_i, Y_i / R)$ and open interest of $SO_i = Y_i - min(Y_i, Y_i / R)$

# TODO
- [X] Rearrange the order, we can run the Interest Match algorithm only on Block Start and not on Block End.
- [ ] Add funding mechanism.
- [ ] Add interest for collateral.
- [ ] Format the document.
