## SVM rollup
As part of the Turbin3 SVM cohort, our team built our own SVM rollup.
It fetches transactions, delegates funds via a Solana program, sends transactions to a sequencer, and processes them by locking accounts, loads and executes transactions, updating local state, and bundling similar transactions. Once a batch threshold is met (e.g., after 10 transactions), the rollup bundles them into one and settles the changes back on-chain.

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
7. **Batch(10) Bundling:** After 10 transactions, bundles them into a single transaction.
8. **Settles Changes to the Chain:** Commits batched changes back to Solana.
