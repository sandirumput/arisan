//   Copyright 2026 sandirumput
//   SPDX-License-Identifier: MIT

//! Arisan — a rotating savings circle (ROSCA) on Tari Ootle.
//!
//! An *arisan* is how millions of people in Indonesia (and across Southeast
//! Asia, Africa and Latin America) save together: a fixed group of friends
//! each puts in the same amount every round, and each round one member — in
//! turn — takes the whole pot home. No bank, no interest, no paperwork.
//!
//! The trust problem with a real-world arisan is the organiser: they hold the
//! pot, and they can disappear with it — or an early winner can stop paying
//! once they have had their turn. This template removes the organiser
//! entirely:
//!
//! * The pot lives in the component vault; nobody can withdraw from it except
//!   through `payout` / `settle_default`, which only ever pay members.
//! * Payout order is fixed at join time (first-come, first-served) and is
//!   public on-chain, so everyone can see whose turn is next.
//! * Every round has an epoch deadline. If someone stops paying, anyone can
//!   call `settle_default`: the defaulter is expelled, everyone who paid that
//!   round is refunded in full, and the circle continues with the members who
//!   are still paying. A member who already took their pot and then defaults
//!   cannot take anything else — the most they can do is shrink future pots
//!   by one contribution.
//! * There is no admin key and no fee. The organiser can only `start` the
//!   circle; after that the rules run themselves.
//!
//! Amounts are in micro-Tari (µT): 1 TARI = 1,000,000 µT.

use tari_template_lib::prelude::*;

#[template]
mod arisan {
    use std::collections::HashMap;

    use super::*;

    /// One rotating savings circle.
    ///
    /// Members join in order; that order IS the payout order (round `r` pays
    /// `members[r]`). The circle runs for exactly `members.len()` rounds and
    /// then completes.
    pub struct ArisanCircle {
        /// The pot for the current round. Only ever holds TARI.
        pot_vault: Vault,
        /// Fixed contribution per member per round, in micro-Tari.
        contribution: Amount,
        /// Maximum number of members (set at creation).
        max_members: u32,
        /// How many epochs each round lasts before it can be settled as defaulted.
        round_length_epochs: u64,
        /// The organiser (component creator). Only they can `start` the circle.
        organizer: RistrettoPublicKeyBytes,
        /// Members in join order = payout order.
        members: Vec<RistrettoPublicKeyBytes>,
        /// Where each member wants their pot (or refund) paid.
        payout_to: HashMap<RistrettoPublicKeyBytes, ComponentManager>,
        /// Members who have paid in the current round.
        paid_this_round: Vec<RistrettoPublicKeyBytes>,
        /// Current round index (0-based). Round `r` pays `members[r]`.
        round: u32,
        /// Epoch by which the current round's contributions are due.
        round_deadline: u64,
        /// True once `start` has been called.
        started: bool,
        /// True once every member has received the pot, or the circle dissolved.
        completed: bool,
    }

    impl ArisanCircle {
        /// Creates a new circle. The creator becomes member #0 (first payout).
        ///
        /// # Panics
        ///
        /// Panics if the contribution is zero, fewer than 2 members are
        /// allowed, or the round length is zero.
        pub fn new(
            contribution: Amount,
            max_members: u32,
            round_length_epochs: u64,
            payout_to: ComponentAddress,
        ) -> Component<Self> {
            assert!(!contribution.is_zero(), "Contribution must be greater than zero");
            assert!(max_members >= 2, "A circle needs at least 2 members");
            assert!(round_length_epochs > 0, "Round length must be greater than zero");

            let organizer = CallerContext::transaction_signer_public_key();
            let mut members = Vec::new();
            members.push(organizer);
            let mut payout_to_map = HashMap::new();
            payout_to_map.insert(organizer, ComponentManager::get(payout_to));

            // Anyone may join, contribute, trigger a payout or settle a
            // default. `start` stays owner-only (the organiser), and the
            // read-only views are open to everyone.
            let access_rules = ComponentAccessRules::new()
                .method("join", rule![allow_all])
                .method("contribute", rule![allow_all])
                .method("payout", rule![allow_all])
                .method("settle_default", rule![allow_all])
                .method("member_count", rule![allow_all])
                .method("current_round", rule![allow_all])
                .method("paid_count", rule![allow_all])
                .method("pot_balance", rule![allow_all])
                .method("is_started", rule![allow_all])
                .method("is_completed", rule![allow_all])
                .method("has_paid", rule![allow_all])
                .method("is_member", rule![allow_all]);

            Component::new(Self {
                pot_vault: Vault::new_empty(TARI_TOKEN),
                contribution,
                max_members,
                round_length_epochs,
                organizer,
                members,
                payout_to: payout_to_map,
                paid_this_round: Vec::new(),
                round: 0,
                round_deadline: 0,
                started: false,
                completed: false,
            })
            .with_access_rules(access_rules)
            .create()
        }

        /// Join the circle. Only while it is still open (not started) and not
        /// full. Join order decides payout order, so joining early means an
        /// earlier pot.
        ///
        /// Callable by: anyone
        ///
        /// # Panics
        ///
        /// Panics if the circle has started, is full, or the caller is already
        /// a member.
        pub fn join(&mut self, payout_to: ComponentAddress) {
            assert!(!self.started, "Circle already started");
            assert!(!self.completed, "Circle is completed");
            assert!(
                (self.members.len() as u32) < self.max_members,
                "Circle is full"
            );
            let member = CallerContext::transaction_signer_public_key();
            assert!(!self.members.contains(&member), "Already a member");
            self.members.push(member);
            self.payout_to.insert(member, ComponentManager::get(payout_to));
            emit_event(
                "MemberJoined",
                metadata![
                    "member" => member.to_string(),
                    "member_count" => self.members.len().to_string(),
                ],
            );
        }

        /// Start the circle. Needs at least 2 members. Opens round 0 with a
        /// deadline `round_length_epochs` from now.
        ///
        /// Callable by: the organiser (component owner) only
        ///
        /// # Panics
        ///
        /// Panics if the circle has already started or has fewer than 2 members.
        pub fn start(&mut self) {
            assert!(!self.started, "Circle already started");
            assert!(self.members.len() >= 2, "Need at least 2 members to start");
            self.started = true;
            self.round_deadline = Consensus::current_epoch() + self.round_length_epochs;
            emit_event(
                "CircleStarted",
                metadata![
                    "organizer" => self.organizer.to_string(),
                    "members" => self.members.len().to_string(),
                    "contribution" => self.contribution.to_string(),
                    "round_deadline" => self.round_deadline.to_string(),
                ],
            );
        }

        /// Pay this round's contribution. Exactly `contribution` TARI, once per
        /// round, from a member.
        ///
        /// Callable by: anyone (membership is checked against the signer)
        ///
        /// # Panics
        ///
        /// Panics if the circle has not started or is completed, the caller is
        /// not a member, the caller already paid this round, the payment is not
        /// TARI, or the amount is not exactly the contribution.
        pub fn contribute(&mut self, payment: Bucket) {
            assert!(self.started, "Circle has not started");
            assert!(!self.completed, "Circle is completed");
            let member = CallerContext::transaction_signer_public_key();
            assert!(self.members.contains(&member), "Not a member of this circle");
            assert!(
                !self.paid_this_round.contains(&member),
                "Already paid this round"
            );
            assert_eq!(
                payment.resource_address(),
                TARI_TOKEN,
                "Contributions must be paid in TARI"
            );
            assert_eq!(
                payment.amount(),
                self.contribution,
                "Contribution must be exactly the circle amount"
            );
            self.pot_vault.deposit(payment);
            self.paid_this_round.push(member);
            emit_event(
                "Contributed",
                metadata![
                    "member" => member.to_string(),
                    "round" => self.round.to_string(),
                    "paid_count" => self.paid_this_round.len().to_string(),
                    "member_count" => self.members.len().to_string(),
                ],
            );
        }

        /// Pay the pot to this round's member. Anyone can call it once every
        /// member has paid. The full pot (members x contribution) goes to
        /// `members[round]`'s payout account, the round advances, and the
        /// circle completes after the last member has been paid.
        ///
        /// Callable by: anyone
        ///
        /// # Panics
        ///
        /// Panics if the circle has not started or is completed, or if not all
        /// members have paid this round.
        pub fn payout(&mut self) {
            assert!(self.started, "Circle has not started");
            assert!(!self.completed, "Circle is completed");
            assert_eq!(
                self.paid_this_round.len(),
                self.members.len(),
                "Not all members have paid this round"
            );

            let recipient = self.members[self.round as usize];
            let pot = self.pot_vault.withdraw(self.pot_vault.balance());
            let pot_amount = pot.amount();
            let payout_to = self
                .payout_to
                .get(&recipient)
                .expect("member must have a payout account")
                .clone();
            payout_to.invoke("deposit", args![pot]);

            emit_event(
                "PotPaid",
                metadata![
                    "recipient" => recipient.to_string(),
                    "round" => self.round.to_string(),
                    "amount" => pot_amount.to_string(),
                ],
            );

            self.paid_this_round.clear();
            self.round += 1;
            if (self.round as usize) >= self.members.len() {
                self.completed = true;
                emit_event("CircleCompleted", metadata![]);
            } else {
                self.round_deadline =
                    Consensus::current_epoch() + self.round_length_epochs;
            }
        }

        /// Settle a round where the deadline has passed and not everyone paid.
        /// Unpaid members are expelled, everyone who paid this round is
        /// refunded in full, and the circle continues with the remaining
        /// members (the round index shifts back past any expelled member who
        /// had already received their pot, so nobody's turn is skipped).
        /// If fewer than 2 members remain — or every remaining member
        /// has already received the pot — the circle dissolves (everyone has
        /// already been refunded for the open round).
        ///
        /// Callable by: anyone
        ///
        /// # Panics
        ///
        /// Panics if the circle has not started or is completed, the deadline
        /// has not passed yet, or everyone has already paid (call `payout`
        /// instead).
        pub fn settle_default(&mut self) {
            assert!(self.started, "Circle has not started");
            assert!(!self.completed, "Circle is completed");
            assert!(
                Consensus::current_epoch() >= self.round_deadline,
                "Round deadline has not passed yet"
            );
            assert!(
                self.paid_this_round.len() < self.members.len(),
                "Everyone has paid - call payout instead"
            );

            // Refund every member who paid this round, in full.
            let paid = std::mem::take(&mut self.paid_this_round);
            for member in &paid {
                let refund = self.pot_vault.withdraw(self.contribution);
                let payout_to = self
                    .payout_to
                    .get(member)
                    .expect("member must have a payout account")
                    .clone();
                payout_to.invoke("deposit", args![refund]);
            }

            // Expel everyone who did not pay. Members who sat *before* the
            // current round already received their pot, so expelling one of
            // them shifts everyone after them one slot earlier: the round
            // index moves back by that count so the next member in line
            // still gets their turn. (Without this, expelling an early
            // recipient would skip the member whose turn is next.)
            let current_round = self.round as usize;
            let mut kept = Vec::new();
            let mut expelled = 0u32;
            let mut expelled_before_round = 0u32;
            for (index, member) in self.members.iter().enumerate() {
                if paid.contains(member) {
                    kept.push(*member);
                } else {
                    self.payout_to.remove(member);
                    expelled += 1;
                    if index < current_round {
                        expelled_before_round += 1;
                    }
                }
            }
            self.members = kept;
            self.round -= expelled_before_round;
            emit_event(
                "MembersExpelled",
                metadata![
                    "round" => self.round.to_string(),
                    "expelled" => expelled.to_string(),
                    "remaining" => self.members.len().to_string(),
                ],
            );

            // The circle can only continue with 2+ members who still have a
            // pot coming. If the current round index is now past the end of
            // the (shrunk) member list, every remaining member has already
            // received their pot.
            if self.members.len() < 2 || (self.round as usize) >= self.members.len() {
                self.completed = true;
                emit_event("CircleCompleted", metadata!["reason" => "dissolved"]);
            } else {
                self.round_deadline =
                    Consensus::current_epoch() + self.round_length_epochs;
            }
        }

        // ---- Read-only views (anyone) ----

        pub fn member_count(&self) -> u32 {
            self.members.len() as u32
        }

        pub fn current_round(&self) -> u32 {
            self.round
        }

        pub fn paid_count(&self) -> u32 {
            self.paid_this_round.len() as u32
        }

        pub fn pot_balance(&self) -> Amount {
            self.pot_vault.balance()
        }

        pub fn is_started(&self) -> bool {
            self.started
        }

        pub fn is_completed(&self) -> bool {
            self.completed
        }

        pub fn has_paid(&self, member: RistrettoPublicKeyBytes) -> bool {
            self.paid_this_round.contains(&member)
        }

        pub fn is_member(&self, member: RistrettoPublicKeyBytes) -> bool {
            self.members.contains(&member)
        }
    }
}
