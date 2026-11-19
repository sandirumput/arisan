// Arisan — walletless browser simulation of the ArisanCircle Ootle template.
//
// This mirrors the Rust template's rules and rejection messages exactly, so
// anyone can walk through the full product in a browser with no wallet, no
// funds and no network. Two things it cannot mirror: on-chain, `contribute`
// also verifies the payment bucket is TARI (here amounts are plain numbers),
// and the engine's access-denied message names the full component action
// (here it is shortened to the method name). Members are identified by name
// here; on-chain they are transaction signer public keys.
//
// Amounts are in micro-Tari (µT): 1 TARI = 1,000,000 µT.

export class ArisanError extends Error {}

function fail(message) {
  throw new ArisanError(message);
}

export class ArisanSim {
  /**
   * @param {object} opts
   * @param {number} opts.contribution - fixed contribution per member per round (µT)
   * @param {number} opts.maxMembers - maximum members (>= 2)
   * @param {number} opts.roundLengthEpochs - epochs per round (> 0)
   * @param {string} opts.organizer - name of the creator (member #0)
   */
  constructor({ contribution, maxMembers, roundLengthEpochs, organizer }) {
    if (!contribution || contribution <= 0) fail("Contribution must be greater than zero");
    if (maxMembers < 2) fail("A circle needs at least 2 members");
    if (!roundLengthEpochs || roundLengthEpochs <= 0) fail("Round length must be greater than zero");

    this.contribution = contribution;
    this.maxMembers = maxMembers;
    this.roundLengthEpochs = roundLengthEpochs;
    this.organizer = organizer;
    this.members = [organizer];
    this.payoutTo = new Map([[organizer, organizer]]);
    this.paidThisRound = [];
    this.round = 0;
    this.roundDeadline = 0;
    this.started = false;
    this.completed = false;
    this.pot = 0;
    this.epoch = 0;
    // Demo-only ledger: starting balance per member, so the UI can show
    // balances moving. On-chain these are real account vaults.
    this.balances = new Map([[organizer, 1_000_000_000]]);
    this.events = [];
    this.emit("CircleCreated", {
      organizer,
      members: "1",
      contribution: String(contribution),
    });
  }

  emit(topic, payload) {
    this.events.push({ topic: `ArisanCircle.${topic}`, payload });
  }

  // Demo helper: move the chain clock forward (on-chain this happens as
  // epochs pass; ~20 min per epoch on mainnet).
  advanceEpoch(epochs = 1) {
    this.epoch += epochs;
    return this.epoch;
  }

  join(member) {
    if (this.started) fail("Circle already started");
    if (this.completed) fail("Circle is completed");
    if (this.members.length >= this.maxMembers) fail("Circle is full");
    if (this.members.includes(member)) fail("Already a member");
    this.members.push(member);
    this.payoutTo.set(member, member);
    if (!this.balances.has(member)) this.balances.set(member, 1_000_000_000);
    this.emit("MemberJoined", {
      member,
      member_count: String(this.members.length),
    });
  }

  start(caller) {
    if (caller !== this.organizer) {
      fail("Access Denied: You must be the owner to perform this action: start");
    }
    if (this.started) fail("Circle already started");
    if (this.members.length < 2) fail("Need at least 2 members to start");
    this.started = true;
    this.roundDeadline = this.epoch + this.roundLengthEpochs;
    this.emit("CircleStarted", {
      organizer: this.organizer,
      members: String(this.members.length),
      contribution: String(this.contribution),
      round_deadline: String(this.roundDeadline),
    });
  }

  contribute(member, amount) {
    if (!this.started) fail("Circle has not started");
    if (this.completed) fail("Circle is completed");
    if (!this.members.includes(member)) fail("Not a member of this circle");
    if (this.paidThisRound.includes(member)) fail("Already paid this round");
    if (amount !== this.contribution) fail("Contribution must be exactly the circle amount");
    this.balances.set(member, this.balances.get(member) - amount);
    this.pot += amount;
    this.paidThisRound.push(member);
    this.emit("Contributed", {
      member,
      round: String(this.round),
      paid_count: String(this.paidThisRound.length),
      member_count: String(this.members.length),
    });
  }

  payout() {
    if (!this.started) fail("Circle has not started");
    if (this.completed) fail("Circle is completed");
    if (this.paidThisRound.length !== this.members.length) {
      fail("Not all members have paid this round");
    }
    const recipient = this.members[this.round];
    const potAmount = this.pot;
    this.pot = 0;
    this.balances.set(recipient, this.balances.get(recipient) + potAmount);
    this.emit("PotPaid", {
      recipient,
      round: String(this.round),
      amount: String(potAmount),
    });
    this.paidThisRound = [];
    this.round += 1;
    if (this.round >= this.members.length) {
      this.completed = true;
      this.emit("CircleCompleted", {});
    } else {
      this.roundDeadline = this.epoch + this.roundLengthEpochs;
    }
    return { recipient, amount: potAmount };
  }

  settleDefault() {
    if (!this.started) fail("Circle has not started");
    if (this.completed) fail("Circle is completed");
    if (this.epoch < this.roundDeadline) fail("Round deadline has not passed yet");
    if (this.paidThisRound.length >= this.members.length) {
      fail("Everyone has paid - call payout instead");
    }

    // Refund every member who paid this round, in full.
    const paid = [...this.paidThisRound];
    this.paidThisRound = [];
    for (const member of paid) {
      this.pot -= this.contribution;
      this.balances.set(member, this.balances.get(member) + this.contribution);
    }

    // Expel everyone who did not pay; shift the round index back past any
    // expelled member who had already received their pot.
    const currentRound = this.round;
    const kept = [];
    let expelled = 0;
    let expelledBeforeRound = 0;
    this.members.forEach((member, index) => {
      if (paid.includes(member)) {
        kept.push(member);
      } else {
        this.payoutTo.delete(member);
        expelled += 1;
        if (index < currentRound) expelledBeforeRound += 1;
      }
    });
    this.members = kept;
    this.round -= expelledBeforeRound;
    this.emit("MembersExpelled", {
      round: String(this.round),
      expelled: String(expelled),
      remaining: String(this.members.length),
    });

    if (this.members.length < 2 || this.round >= this.members.length) {
      this.completed = true;
      this.emit("CircleCompleted", { reason: "dissolved" });
    } else {
      this.roundDeadline = this.epoch + this.roundLengthEpochs;
    }
    return { expelled, remaining: this.members.length };
  }

  // ---- Read-only views (mirror the template) ----

  memberCount() {
    return this.members.length;
  }

  currentRound() {
    return this.round;
  }

  paidCount() {
    return this.paidThisRound.length;
  }

  potBalance() {
    return this.pot;
  }

  isStarted() {
    return this.started;
  }

  isCompleted() {
    return this.completed;
  }

  hasPaid(member) {
    return this.paidThisRound.includes(member);
  }

  isMember(member) {
    return this.members.includes(member);
  }

  balanceOf(member) {
    return this.balances.get(member) ?? 0;
  }

  currentRecipient() {
    return this.completed ? null : this.members[this.round];
  }

  /** Plain-object snapshot for the UI. */
  snapshot() {
    return {
      contribution: this.contribution,
      maxMembers: this.maxMembers,
      roundLengthEpochs: this.roundLengthEpochs,
      organizer: this.organizer,
      members: [...this.members],
      paidThisRound: [...this.paidThisRound],
      round: this.round,
      roundDeadline: this.roundDeadline,
      started: this.started,
      completed: this.completed,
      pot: this.pot,
      epoch: this.epoch,
      recipient: this.currentRecipient(),
      balances: Object.fromEntries(this.balances),
      events: this.events.map((e) => ({ ...e })),
    };
  }
}

/** Format µT as a readable TARI amount (1 TARI = 1,000,000 µT). */
export function formatTari(microTari) {
  const whole = Math.floor(microTari / 1_000_000);
  const frac = String(microTari % 1_000_000).padStart(6, "0").replace(/0+$/, "");
  return frac ? `${whole}.${frac} TARI` : `${whole} TARI`;
}
