# LeadGen Contracts

Trust-minimized escrow contract for freelance payments, plus Hardhat tooling.

## Contract: `FreelanceEscrow.sol`

State machine:

```
Funded(0)  → startWork()  → InProgress(1)  → submitWork()  → Submitted(2)
Submitted(2)  → approve()  → Completed(3)   (freelancer paid)
InProgress(1) | Submitted(2) → raiseDispute() → Disputed(4)
Disputed(4)   → resolveDispute(shareEth) → Completed(3)  (mediator splits funds)
Funded(0)     → cancelBeforeWork() → Refunded(5)
after deadline → refundAfterDeadline() → Refunded(5)
```

### Roles

- **client** — the deployer; deposits `msg.value` and approves work
- **freelancer** — named in the constructor; starts and submits work
- **mediator** — optional (`address(0)` to disable); resolves disputes by splitting funds

### Security

- Only EOAs; no ERC20; guarded ETH sends (`TransferFailed` reverts)
- Zero-address / zero-amount checks in constructor
- Non-party and wrong-state access are reverted with custom errors

## Scripts

```bash
npm install
npm run compile          # compile the contract
npm run test             # run Hardhat test suite (9 tests)
npm run node             # start a local Hardhat node
npm run deploy:local     # deploy to localhost node
npm run deploy:sepolia   # deploy to Sepolia
npm run deploy:mainnet   # deploy to mainnet
```

## Deploying to a live network

1. `cp .env.example .env` and fill in `SEPOLIA_RPC_URL` / `MAINNET_RPC_URL`, `PRIVATE_KEY`, and optionally `FREELANCER_ADDRESS` / `MEDIATOR_ADDRESS` / `AMOUNT_ETH`.
2. Run the deploy script and note the printed address.
3. Optionally verify with `npx hardhat verify <address> --network sepolia` using `ETHERSCAN_API_KEY`.

> Never commit the `.env` file (it is gitignored).

## Integration

The compiled artifact's ABI + bytecode are mirrored to `frontend/src/lib/FreelanceEscrow.json`, which the frontend uses to deploy from the connected wallet. After a source change, re-run `npm run compile` and re-copy the artifact.
