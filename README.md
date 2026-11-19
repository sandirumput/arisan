# Arisan — rotating savings circles on Tari Ootle

**Entry for the Tari November Build Contest (MVP). MIT licensed.**

An *arisan* is how millions of people in Indonesia — and across Southeast
Asia, Africa and Latin America — save together: a fixed group of friends each
puts in the same amount every round, and each round one member, in turn, takes
the whole pot home. No bank, no interest, no paperwork.

The trust problem with a real-world arisan is the organiser: they hold the
pot, and they can disappear with it — or an early winner can stop paying once
they have had their turn. **Arisan puts the whole circle on the Ootle, so the
organiser holds nothing:**

- The pot lives in the component vault. Nobody can withdraw from it except
  through `payout` / `settle_default`, which only ever pay members.
- Payout order is fixed at join time (first-come, first-served) and is public
  on-chain, so everyone can see whose turn is next.
- Every round has an epoch deadline. If someone stops paying, **anyone** can
  call `settle_default`: the defaulter is expelled, everyone who paid that
  round is refunded in full, and the circle continues with the members who are
  still paying. A member who already took their pot and then defaults cannot
  take anything else — the most they can do is shrink future pots by one
  contribution each.
- No admin key, no fee. The organiser can only `start` the circle; after that
  the rules run themselves.

## How a circle runs

1. **Create** — set the contribution (µT), max members (≥ 2) and round length
   (epochs). The creator becomes member #0 and takes the first pot.
2. **Join** — members join while the circle is open. Join order = payout order.
3. **Start** — the organiser starts it (needs ≥ 2 members). Round 0 opens with
   a deadline `round_length_epochs` from now.
4. **Contribute** — each member pays exactly the contribution, once per round.
   Wrong amounts are rejected and charge nothing.
5. **Payout** — once everyone has paid, anyone calls `payout` and the full pot
   (members × contribution) goes to that round's member. The circle completes
   after the last member has been paid.
6. **Default** — if the deadline passes with unpaid members, anyone calls
   `settle_default`: payers refunded, defaulters expelled, circle continues
   (or dissolves if fewer than 2 members remain).

Amounts are in micro-Tari (µT): 1 TARI = 1,000,000 µT. A mainnet epoch is
~20 minutes, so a round length of 72 epochs ≈ 1 day.

## What's in this repo

| Path | What it is |
| --- | --- |
| `src/lib.rs` | The `ArisanCircle` Rust `#[template]` (`tari_template_lib` 0.40) |
| `tests/test.rs` | 11 engine tests against the real Ootle engine in-process |
| `app/index.html` | Walletless browser demo (no wallet, no funds, no network) |
| `app/sim.mjs` | The demo's simulation engine — mirrors the template's rules and rejection messages |
| `app/sim.test.mjs` | 11 parity tests: the sim cannot drift from the template |
| `demo-output.txt` | Captured output of the test runs and the WASM build |

## Try it

**Browser demo (fastest):** open `app/index.html` — or the hosted demo linked
in the contest post. Press **▶ Run guided walkthrough** for the full story in
~3 minutes: a full round, a rejected wrong-amount payment, a member who stops
paying, the default settlement, and the circle completing.

**Verify the template (no network, no funds needed):**

```sh
cargo test                       # 11 engine tests, real Ootle engine
node --test app/sim.test.mjs     # 11 simulation parity tests
cargo build --target wasm32-unknown-unknown --release   # WASM binary
```

The engine tests assert exact balances on every path, including the
zero-sum invariant: after a full circle with no defaults, every member ends
with exactly their starting balance.

## Honest limits

- The template has **not** been published to Esmeralda or mainnet and is **not
  audited**. The demo is a simulation and says so on the page.
- Contribution amounts and payout order are **public** on-chain (vault
  accounting), like any Ootle vault deposit. Hiding them would need a
  confidential/stealth design.
- No code can force a human to keep paying: a member who takes an early pot
  and defaults keeps that pot (minus their own contribution). The mechanism
  bounds the loss to one contribution per remaining member per remaining
  round — it does not prevent it.
- The payout order is fixed at join time; there is no lottery or bidding for
  turns yet. (A random-order variant using the engine's deterministic
  randomness is the obvious next step.)
- Round deadlines are in epochs, so a round's wall-clock length depends on
  epoch time (~20 min on mainnet, ~1 h on Esmeralda).

## Roadmap (if it wins / after the contest)

1. Publish the template to Esmeralda, then mainnet; wire the demo to the
   Sapient wallet for a live path alongside the simulation.
2. Random payout order (commit-reveal or engine randomness) as a circle option.
3. Late-payment grace: let a member pay late (before settlement) instead of
   only being expelled.

## Development

Built with an AI coding agent under the owner's direction, as the contest
rules allow. The code is original, written for this round, and MIT licensed.

Template scaffold: the official
[`wasm-template`](https://github.com/tari-project/wasm-template) `empty`
template (`tari_template_lib` 0.40 / `tari_template_test_tooling` 0.53).
Rust edition 2024; WASM target `wasm32-unknown-unknown`.
