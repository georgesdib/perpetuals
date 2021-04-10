# PolkaSynthetics
Platform for synthetic derivatives on Polkadot/Acala

# Minting
Buyer A expresses interest in buying W of asset AA.
A wires 20% in IM, and opens interest.
Buyer B expresses interest in buying X of asset AA, wires IM as well.
Seller C expresses interest in selling Y of asset AA.
C wires 20% in IM, and opens interest.
Seller D expresses interest in selling Z of asset AA, wires IM as well.

# End of block
## Interest match
If X = W = 0 then no interest to match.
Call P = (Z+Y) / (X+W)
A has bought min(W, W * P).
B has bought min(X, X * P).
C has sold min(Y, Y / P).
D has sold min(Z, Z / P).
