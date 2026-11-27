## Arisan — rotating savings circles on Tari Ootle

**Code (MIT):** https://github.com/sandirumput/arisan

**Play demo (no wallet):** https://sandirumput.github.io/arisan/

**Tari payment address:** 124Y6c9juhdo6CCk3z4KkJFDyvTsacsN17rFTMhjxuCzdx7iiK4wqRZL8czskx7yUsiP5ww1CGHhydYTZvA2piLoLpC

**Announcement:** https://medium.com/@sandirumput0415/arisan-rotating-savings-circles-on-tari-ootle-d200c54b742a

**What it is**

An *arisan* is how millions of people in Indonesia — and across Southeast
Asia, Africa and Latin America — save together: a fixed group of friends each
puts in the same amount every round, and each round one member, in turn, takes
the whole pot home. No bank, no interest, no paperwork.

The trust problem with a real-world arisan is the organiser: they hold the
pot, and they can disappear with it — or an early winner can stop paying once
they have had their turn. Arisan puts the whole circle on the Ootle, so the
organiser holds nothing.

**How it works**

- Create a circle: fixed contribution, max members, round length in epochs.
  The creator becomes member #0 and takes the first pot.
- Members join while the circle is open. Join order *is* payout order, public
  on-chain, so everyone can see whose turn is next.
- Each round, every member pays exactly the contribution into the pot vault.
  Wrong amounts are rejected and charge nothing.
- Once everyone has paid, anyone calls `payout` and the full pot goes to that
  round's member. The circle completes after the last member has been paid.
- Every round has an epoch deadline. If someone stops paying, anyone can call
  `settle_default`: the defaulter is expelled, everyone who paid that round is
  refunded in full, and the circle continues with the members who are still
  paying. The round index shifts back past any expelled member who had already
  received their pot, so nobody's turn is skipped. If fewer than 2 members
  remain, the circle dissolves with everyone refunded.
- No admin key, no fee. The organiser can only `start` the circle; after that
  the rules run themselves.

**What's built and tested**

- Rust `#[template]` on `tari_template_lib` 0.40, compiled to
  `wasm32-unknown-unknown` (146 KiB release build).
- 11 engine tests with `tari_template_test_tooling` 0.53 against the real
  Ootle engine, asserting exact balances on every path — including the
  zero-sum invariant (a full circle with no defaults leaves every member with
  exactly their starting balance) and the classic arisan attack (the first
  recipient takes the pot, then stops paying: bounded to one contribution per
  remaining member, and they can never touch another pot).
- Walletless browser demo whose simulation mirrors the template's rules and
  rejection messages exactly, with 11 parity tests (`node --test
  app/sim.test.mjs`) and a guided 3-minute walkthrough — including the
  deliberate rejections and a full default settlement.

**How to verify (for judges)**

1. Open the demo link, press ▶ Run guided walkthrough.
2. Clone the repo, run `cargo test` (11 engine tests, no network needed).
3. Run `node --test app/sim.test.mjs` (11 simulation parity tests).
4. Build the WASM: `cargo build --target wasm32-unknown-unknown --release`.

**Honest limits.** The template has not been published to Esmeralda or
mainnet yet and is not audited; the demo is a simulation and says so on the
page. Contribution amounts and payout order are public on-chain (vault
accounting) — hiding them would need a confidential/stealth design. No code
can force a human to keep paying: a member who takes an early pot and
defaults keeps that pot minus their own contribution; the mechanism bounds
the loss, it does not prevent it. Payout order is fixed at join time — a
random-order variant is the next step.

Built with an AI coding agent under my direction, as the rules allow. The
code is original, written for this round, and MIT licensed.

