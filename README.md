## SVM rollup
As part of the Turbin3 SVM cohort, our team built our own SVM rollup.

## Overview
A rollup is a Layer-2 scaling solution that processes and bundles transactions off-chain before settling them on the main chain. This reduces congestion and fees while keeping the security of the underlying blockchain.

## Why would Solana need a rollup?
Rollups can enhance Solana by:
- **Increasing Throughput:** Offload transactions from the main chain.
- **Lower fees:** Batch transactions to lower costs.
- **Flexibility:** Allow for customized transaction processing without changing Solana’s core.

## Flow
1. **Fetches Transactions** 
2. **Delegates Funds:** Solana program
3. **Sends to the Sequencer**
4. **Locks accounts, Loads, and executes Transactions** 
5. **Updates Local State** 
6. **Bundles Similar Transactions:** Groups similar transactions into one.
7. **Batch Bundling:** After 10 transactions, bundles them into a single unit.
8. **Settles Changes to the Chain:** Commits batched changes back to Solana.
