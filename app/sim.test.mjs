// Parity tests for the Arisan browser simulation (app/sim.mjs).
// Each test mirrors a Rust engine test in tests/test.js — same scenario, same
// expected outcome — so the demo cannot drift from the template.
// Run: node --test app/sim.test.mjs

import test from "node:test";
import assert from "node:assert/strict";
import { ArisanSim, ArisanError } from "./sim.mjs";

const CONTRIBUTION = 100;
const ROUND_LEN = 10;
const INITIAL = 1_000_000_000;

function circle3() {
  const sim = new ArisanSim({
    contribution: CONTRIBUTION,
    maxMembers: 4,
    roundLengthEpochs: ROUND_LEN,
    organizer: "Ana",
  });
  sim.join("Budi");
  sim.join("Citra");
  sim.start("Ana");
  return sim;
}

function assertThrowsWith(fn, message) {
  assert.throws(fn, (err) => err instanceof ArisanError && err.message === message);
}

test("full circle completes and everyone ends whole", () => {
  const sim = circle3();
  const members = ["Ana", "Budi", "Citra"];

  for (let round = 0; round < 3; round += 1) {
    for (const m of members) sim.contribute(m, CONTRIBUTION);
    assert.equal(sim.paidCount(), 3);
    assert.equal(sim.potBalance(), 300);
    const before = sim.balanceOf(members[round]);
    const { recipient, amount } = sim.payout();
    assert.equal(recipient, members[round]);
    assert.equal(amount, 300);
    assert.equal(sim.balanceOf(members[round]), before + 300);
    assert.equal(sim.currentRound(), round + 1);
  }

  assert.equal(sim.isCompleted(), true);
  assert.equal(sim.potBalance(), 0);
  for (const m of members) assert.equal(sim.balanceOf(m), INITIAL);
});

test("payout emits a PotPaid event", () => {
  const sim = new ArisanSim({
    contribution: CONTRIBUTION,
    maxMembers: 2,
    roundLengthEpochs: ROUND_LEN,
    organizer: "Ana",
  });
  sim.join("Budi");
  sim.start("Ana");
  sim.contribute("Ana", CONTRIBUTION);
  sim.contribute("Budi", CONTRIBUTION);
  sim.payout();
  const event = sim.events.find((e) => e.topic === "ArisanCircle.PotPaid");
  assert.ok(event);
  assert.equal(event.payload.amount, "200");
  assert.equal(event.payload.round, "0");
});

test("join rules are enforced", () => {
  const sim = new ArisanSim({
    contribution: CONTRIBUTION,
    maxMembers: 3,
    roundLengthEpochs: ROUND_LEN,
    organizer: "Ana",
  });
  assertThrowsWith(() => sim.join("Ana"), "Already a member");
  sim.join("Budi");
  sim.join("Citra");
  assertThrowsWith(() => sim.join("Dewi"), "Circle is full");
  sim.start("Ana");
  assertThrowsWith(() => sim.join("Dewi"), "Circle already started");
});

test("start requires two members and the organiser", () => {
  const sim = new ArisanSim({
    contribution: CONTRIBUTION,
    maxMembers: 4,
    roundLengthEpochs: ROUND_LEN,
    organizer: "Ana",
  });
  assertThrowsWith(() => sim.start("Ana"), "Need at least 2 members to start");
  sim.join("Budi");
  assertThrowsWith(
    () => sim.start("Budi"),
    "Access Denied: You must be the owner to perform this action: start",
  );
  sim.start("Ana");
  assertThrowsWith(() => sim.start("Ana"), "Circle already started");
});

test("contribute rules are enforced", () => {
  const sim = new ArisanSim({
    contribution: CONTRIBUTION,
    maxMembers: 3,
    roundLengthEpochs: ROUND_LEN,
    organizer: "Ana",
  });
  sim.join("Budi");
  assertThrowsWith(() => sim.contribute("Ana", CONTRIBUTION), "Circle has not started");
  sim.start("Ana");
  assertThrowsWith(() => sim.contribute("Dewi", CONTRIBUTION), "Not a member of this circle");
  const before = sim.balanceOf("Ana");
  assertThrowsWith(
    () => sim.contribute("Ana", CONTRIBUTION + 1),
    "Contribution must be exactly the circle amount",
  );
  assert.equal(sim.balanceOf("Ana"), before);
  sim.contribute("Ana", CONTRIBUTION);
  assertThrowsWith(() => sim.contribute("Ana", CONTRIBUTION), "Already paid this round");
});

test("payout requires everyone paid", () => {
  const sim = new ArisanSim({
    contribution: CONTRIBUTION,
    maxMembers: 2,
    roundLengthEpochs: ROUND_LEN,
    organizer: "Ana",
  });
  sim.join("Budi");
  sim.start("Ana");
  sim.contribute("Ana", CONTRIBUTION);
  assertThrowsWith(() => sim.payout(), "Not all members have paid this round");
});

test("default settlement expels the defaulter, refunds payers, and continues", () => {
  const sim = circle3();
  sim.contribute("Ana", CONTRIBUTION);
  sim.contribute("Budi", CONTRIBUTION);
  assertThrowsWith(() => sim.settleDefault(), "Round deadline has not passed yet");

  sim.advanceEpoch(ROUND_LEN);
  const anaBefore = sim.balanceOf("Ana");
  const budiBefore = sim.balanceOf("Budi");
  const { expelled, remaining } = sim.settleDefault();
  assert.equal(expelled, 1);
  assert.equal(remaining, 2);
  assert.equal(sim.balanceOf("Ana"), anaBefore + CONTRIBUTION);
  assert.equal(sim.balanceOf("Budi"), budiBefore + CONTRIBUTION);
  assert.equal(sim.memberCount(), 2);
  assert.equal(sim.isCompleted(), false);
  assert.equal(sim.potBalance(), 0);
  assert.ok(sim.events.some((e) => e.topic === "ArisanCircle.MembersExpelled"));

  // Round 0 restarts with Ana and Budi: Ana takes the pot, then Budi.
  sim.contribute("Ana", CONTRIBUTION);
  sim.contribute("Budi", CONTRIBUTION);
  assert.equal(sim.payout().recipient, "Ana");
  sim.contribute("Ana", CONTRIBUTION);
  sim.contribute("Budi", CONTRIBUTION);
  assert.equal(sim.payout().recipient, "Budi");
  assert.equal(sim.isCompleted(), true);
  assert.equal(sim.balanceOf("Citra"), INITIAL);
});

test("settleDefault rejects when everyone paid", () => {
  const sim = new ArisanSim({
    contribution: CONTRIBUTION,
    maxMembers: 2,
    roundLengthEpochs: ROUND_LEN,
    organizer: "Ana",
  });
  sim.join("Budi");
  sim.start("Ana");
  sim.contribute("Ana", CONTRIBUTION);
  sim.contribute("Budi", CONTRIBUTION);
  sim.advanceEpoch(ROUND_LEN);
  assertThrowsWith(() => sim.settleDefault(), "Everyone has paid - call payout instead");
});

test("circle dissolves when too few members remain", () => {
  const sim = new ArisanSim({
    contribution: CONTRIBUTION,
    maxMembers: 2,
    roundLengthEpochs: ROUND_LEN,
    organizer: "Ana",
  });
  sim.join("Budi");
  sim.start("Ana");
  sim.contribute("Ana", CONTRIBUTION);
  sim.advanceEpoch(ROUND_LEN);
  sim.settleDefault();
  assert.equal(sim.isCompleted(), true);
  assert.equal(sim.balanceOf("Ana"), INITIAL);
  assert.equal(sim.memberCount(), 1);
});

test("constructor validates its arguments", () => {
  assertThrowsWith(
    () =>
      new ArisanSim({ contribution: 0, maxMembers: 3, roundLengthEpochs: 10, organizer: "Ana" }),
    "Contribution must be greater than zero",
  );
  assertThrowsWith(
    () =>
      new ArisanSim({ contribution: 100, maxMembers: 1, roundLengthEpochs: 10, organizer: "Ana" }),
    "A circle needs at least 2 members",
  );
  assertThrowsWith(
    () =>
      new ArisanSim({ contribution: 100, maxMembers: 3, roundLengthEpochs: 0, organizer: "Ana" }),
    "Round length must be greater than zero",
  );
});

test("early recipient who defaults cannot take anything more", () => {
  const sim = circle3();
  // Round 0 completes: Ana receives 300 (net +200).
  for (const m of ["Ana", "Budi", "Citra"]) sim.contribute(m, CONTRIBUTION);
  sim.payout();
  assert.equal(sim.balanceOf("Ana"), INITIAL + 200);

  // Round 1: Ana stops paying; Budi and Citra pay; deadline passes; settle.
  sim.contribute("Budi", CONTRIBUTION);
  sim.contribute("Citra", CONTRIBUTION);
  sim.advanceEpoch(2 * ROUND_LEN);
  sim.settleDefault();
  assert.equal(sim.memberCount(), 2);
  assert.equal(sim.balanceOf("Ana"), INITIAL + 200);
  assert.equal(sim.balanceOf("Budi"), INITIAL - 100);
  assert.equal(sim.balanceOf("Citra"), INITIAL - 100);

  // Circle finishes between Budi and Citra (round shifts back: Budi first).
  sim.contribute("Budi", CONTRIBUTION);
  sim.contribute("Citra", CONTRIBUTION);
  assert.equal(sim.payout().recipient, "Budi");
  sim.contribute("Budi", CONTRIBUTION);
  sim.contribute("Citra", CONTRIBUTION);
  assert.equal(sim.payout().recipient, "Citra");
  assert.equal(sim.isCompleted(), true);
  assert.equal(sim.balanceOf("Budi"), INITIAL - 100);
  assert.equal(sim.balanceOf("Citra"), INITIAL - 100);
});
