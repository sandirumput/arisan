# DRAFT — Tari November Build Contest post (DO NOT POST YET)

Post this as a reply to the contest thread:
https://community.tari.com/t/november-build-contest-thread-mvp/520

Deadline: entries close 1 Dec 2026 00:00 UTC (= 1 Dec 07:00 WIB).
Target: post by 29 Nov to be safe.

Before posting, fill in the four `[FILL IN]` placeholders below (steps at the
bottom of this file). Everything else is ready to paste as-is.

---

## Arisan — rotating savings circles on Tari Ootle

**Code (MIT):** https://github.com/sandirumput/arisan

**Play demo (no wallet):** https://sandirumput.github.io/arisan/

**Tari payment address:** [FILL IN — your Tari (L1 Minotari) address]

**Announcement:** [FILL IN — link to your Bluesky / Reddit / Medium / YouTube announcement post]

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

---

## HUMAN STEPS FOR HOSHI (do these, in order)

### 1. Push the code to GitHub (~10 min, free)
The repo is ready at `~/workspace/makefun/tari-mvp/` (ask nemui for the files
/ a zip if you need them on your PC). On github.com/sandirumput:
- Create a new public repository named `arisan` (no README/gitignore — the
  files already include them).
- Push the contents of `tari-mvp/` to it (or upload via the web UI).
- Repo URL becomes `https://github.com/sandirumput/arisan` — paste it into
  the post above.

### 2. Host the demo free on GitHub Pages (~5 min, free)
- In the repo: Settings → Pages → Source: Deploy from a branch → Branch:
  `main`, folder: `/app` → Save.
- Wait ~1 min. Demo URL becomes `https://sandirumput.github.io/arisan/`
  — open it, press ▶ Run guided walkthrough, check it works, then paste the
  URL into the post above.
- (If the `/app` folder option is not offered, move `index.html` + `sim.mjs`
  to the repo root and use folder `/ (root)` instead.)

### 3. Get a Tari payment address (free, ~10 min)
The prize is paid in XTM on Tari L1 (Minotari), so you need a Tari L1 address
(the long string starting with `12...`, like the other entrants posted).
Easiest free path — Tari Universe mobile wallet:
- Download ONLY from the official site: https://www.tari.com/downloads
  (or the official Tari Universe app on Google Play). Never from random links.
- Open the app → Create a wallet → set a password.
- **Back up the 24-word seed phrase on paper, offline, and never share it
  with anyone — whoever has it owns the wallet.** Settings → Wallet Backups →
  Write Down Seed Phrase.
- Tap Receive → Copy. That long `12...` address is your Tari payment
  address — paste it into the post above.
- Alternative if you prefer desktop: Tari Universe desktop app (same site)
  has a built-in wallet; its Receive button shows the same kind of address.
- Do NOT use an exchange deposit address (MEXC is delisting XTM; exchange
  addresses can change).

### 4. Announce on social media (free, ~10 min)
The contest requires ONE public announcement post on Bluesky, Reddit,
Medium, or YouTube. Easiest: a throwaway Reddit account (the organisers
explicitly accept throwaways — see the rules thread).
- Create a fresh Reddit account (any email works), go to r/tari, and post
  something like: "Arisan — rotating savings circles on Tari Ootle (my
  November Build Contest MVP entry)" + 2–3 sentences + the demo link + the
  GitHub link.
- Copy the post's URL into the post above.
- (Bluesky is the other easy option: free account at bsky.app, one post with
  the demo + repo links.)

### 5. Post in the contest thread (~5 min)
- You need a community.tari.com forum account (free sign-up).
- Reply to https://community.tari.com/t/november-build-contest-thread-mvp/520
  with the filled-in post above. ONE post per submission — if you need to
  update it later, edit that same post.
- Post by **29 Nov** to be safe (hard cutoff 1 Dec 00:00 UTC = 1 Dec 07:00 WIB).

### 6. After posting (optional, free)
- If the judges ask questions in the thread, reply there (or ask nemui to
  draft replies).
- Winners are announced the following month (December) by Council vote.
- If it wins, the XTM goes to the address in step 3. Note: XTM is very
  illiquid — selling a large prize would move the price, so the realisable
  value is below the headline $620.
