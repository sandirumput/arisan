//   Copyright 2026 sandirumput
//   SPDX-License-Identifier: MIT

//! Engine tests for the ArisanCircle template.
//!
//! Every test runs against the real Ootle engine in-process (no network, no
//! funds): `cargo test` reproduces the template's behaviour exactly.

use tari_template_lib::types::{Amount, ComponentAddress, constants::TARI_TOKEN};
use tari_template_test_tooling::{
    TemplateTest,
    crypto::RistrettoSecretKey,
    engine_types::virtual_substate::{VirtualSubstate, VirtualSubstateId},
    support::assert_error::assert_reject_reason,
    transaction::args,
};

const TEMPLATE_NAME: &str = "ArisanCircle";
/// Contribution per member per round, in micro-Tari.
const CONTRIBUTION: u64 = 100;
/// Round length, in epochs.
const ROUND_LEN: u64 = 10;
/// Funded accounts start with 1,000,000,000 µT (1,000 TARI).
const INITIAL_BALANCE: u64 = TemplateTest::FUNDED_ACCOUNT_INITIAL_BALANCE;

struct Member {
    account: ComponentAddress,
    key: RistrettoSecretKey,
}

fn new_member(test: &mut TemplateTest) -> Member {
    let (account, _proof, key) = test.create_funded_account();
    Member { account, key }
}

fn tari_balance(test: &TemplateTest, account: ComponentAddress) -> Amount {
    test.read_only_state_store()
        .get_vaults_for_account(account)
        .unwrap()[&TARI_TOKEN]
        .balance()
}

fn create_circle(
    test: &mut TemplateTest,
    template: tari_template_lib::types::TemplateAddress,
    organizer: &Member,
    max_members: u32,
) -> ComponentAddress {
    test.execute_expect_success(
        test.transaction()
            .call_function(
                template,
                "new",
                args![
                    Amount::new(u128::from(CONTRIBUTION)),
                    max_members,
                    ROUND_LEN,
                    organizer.account
                ],
            )
            .build_and_seal(&organizer.key),
        vec![],
    );
    test.read_only_state_store()
        .get_components_by_template_address(template)
        .unwrap()
        .remove(0)
        .0
}

fn join(test: &mut TemplateTest, circle: ComponentAddress, member: &Member) {
    test.execute_expect_success(
        test.transaction()
            .call_method(circle, "join", args![member.account])
            .build_and_seal(&member.key),
        vec![],
    );
}

fn start(test: &mut TemplateTest, circle: ComponentAddress, organizer: &Member) {
    test.execute_expect_success(
        test.transaction()
            .call_method(circle, "start", args![])
            .build_and_seal(&organizer.key),
        vec![],
    );
}

fn contribute(test: &mut TemplateTest, circle: ComponentAddress, member: &Member, amount: u64) {
    test.execute_expect_success(
        test.transaction()
            .call_method(member.account, "withdraw", args![TARI_TOKEN, amount])
            .put_last_instruction_output_on_workspace("coins")
            .call_method(circle, "contribute", args![Workspace("coins")])
            .build_and_seal(&member.key),
        vec![],
    );
}

fn payout(test: &mut TemplateTest, circle: ComponentAddress, caller: &Member) {
    test.execute_expect_success(
        test.transaction()
            .call_method(circle, "payout", args![])
            .build_and_seal(&caller.key),
        vec![],
    );
}

/// Three friends, three rounds, everyone ends up exactly whole: the pot
/// rotates and the circle completes with zero-sum balances.
#[test]
fn full_circle_completes_and_everyone_ends_whole() {
    let mut test = TemplateTest::my_crate();
    let template = test.get_template_address(TEMPLATE_NAME);

    let ana = new_member(&mut test);
    let budi = new_member(&mut test);
    let citra = new_member(&mut test);
    let members = [&ana, &budi, &citra];

    let circle = create_circle(&mut test, template, &ana, 4);
    join(&mut test, circle, &budi);
    join(&mut test, circle, &citra);
    assert_eq!(test.call_method::<u32>(circle, "member_count", args![], vec![]), 3);

    start(&mut test, circle, &ana);
    assert!(test.call_method::<bool>(circle, "is_started", args![], vec![]));

    // Rounds 0, 1, 2 pay Ana, Budi, Citra in join order.
    for (round, expected_recipient) in members.iter().enumerate() {
        for member in members.iter() {
            contribute(&mut test, circle, member, CONTRIBUTION);
        }
        assert_eq!(
            test.call_method::<u32>(circle, "paid_count", args![], vec![]),
            3,
            "round {round}: everyone should have paid"
        );
        assert_eq!(
            test.call_method::<Amount>(circle, "pot_balance", args![], vec![]),
            Amount::new(300),
            "round {round}: pot should hold 3 x contribution"
        );

        let before = tari_balance(&test, expected_recipient.account);
        payout(&mut test, circle, &ana);
        let after = tari_balance(&test, expected_recipient.account);
        assert_eq!(
            after,
            before + Amount::new(300),
            "round {round}: recipient should receive the full pot"
        );
        assert_eq!(
            test.call_method::<u32>(circle, "current_round", args![], vec![]),
            (round + 1) as u32
        );
    }

    assert!(test.call_method::<bool>(circle, "is_completed", args![], vec![]));
    // Zero-sum: every member paid 3 x 100 and received one pot of 300.
    for member in members {
        assert_eq!(
            tari_balance(&test, member.account),
            Amount::from(INITIAL_BALANCE),
            "member should end with exactly their starting balance"
        );
    }
    // The pot vault is empty at the end.
    assert_eq!(
        test.call_method::<Amount>(circle, "pot_balance", args![], vec![]),
        Amount::new(0)
    );
}

#[test]
fn payout_emits_pot_paid_event() {
    let mut test = TemplateTest::my_crate();
    let template = test.get_template_address(TEMPLATE_NAME);

    let ana = new_member(&mut test);
    let budi = new_member(&mut test);
    let circle = create_circle(&mut test, template, &ana, 2);
    join(&mut test, circle, &budi);
    start(&mut test, circle, &ana);

    contribute(&mut test, circle, &ana, CONTRIBUTION);
    contribute(&mut test, circle, &budi, CONTRIBUTION);

    let result = test.execute_expect_success(
        test.transaction()
            .call_method(circle, "payout", args![])
            .build_and_seal(&budi.key),
        vec![],
    );
    let event = result
        .finalize
        .events
        .iter()
        .find(|e| e.topic() == "ArisanCircle.PotPaid")
        .expect("PotPaid event not found");
    assert_eq!(event.payload().get_str("amount"), Some("200"));
    assert_eq!(event.payload().get_str("round"), Some("0"));
}

#[test]
fn join_rules_are_enforced() {
    let mut test = TemplateTest::my_crate();
    let template = test.get_template_address(TEMPLATE_NAME);

    let ana = new_member(&mut test);
    let budi = new_member(&mut test);
    let citra = new_member(&mut test);
    let dewi = new_member(&mut test);
    let circle = create_circle(&mut test, template, &ana, 3);

    // The organiser is already member #0.
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_method(circle, "join", args![ana.account])
            .build_and_seal(&ana.key),
        vec![],
    );
    assert_reject_reason(reason, "Already a member");

    join(&mut test, circle, &budi);
    join(&mut test, circle, &citra);

    // Full: 3 of 3.
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_method(circle, "join", args![dewi.account])
            .build_and_seal(&dewi.key),
        vec![],
    );
    assert_reject_reason(reason, "Circle is full");

    start(&mut test, circle, &ana);

    // No joining after the start.
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_method(circle, "join", args![dewi.account])
            .build_and_seal(&dewi.key),
        vec![],
    );
    assert_reject_reason(reason, "Circle already started");
}

#[test]
fn start_requires_two_members_and_the_organiser() {
    let mut test = TemplateTest::my_crate();
    let template = test.get_template_address(TEMPLATE_NAME);

    let ana = new_member(&mut test);
    let budi = new_member(&mut test);
    let circle = create_circle(&mut test, template, &ana, 4);

    // Only the organiser is in the circle.
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_method(circle, "start", args![])
            .build_and_seal(&ana.key),
        vec![],
    );
    assert_reject_reason(reason, "Need at least 2 members to start");

    join(&mut test, circle, &budi);

    // A non-organiser cannot start it (owner-only method).
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_method(circle, "start", args![])
            .build_and_seal(&budi.key),
        vec![],
    );
    assert_reject_reason(reason, "Access Denied");

    start(&mut test, circle, &ana);

    // Starting twice fails.
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_method(circle, "start", args![])
            .build_and_seal(&ana.key),
        vec![],
    );
    assert_reject_reason(reason, "Circle already started");
}

#[test]
fn contribute_rules_are_enforced() {
    let mut test = TemplateTest::my_crate();
    let template = test.get_template_address(TEMPLATE_NAME);

    let ana = new_member(&mut test);
    let budi = new_member(&mut test);
    let outsider = new_member(&mut test);
    let circle = create_circle(&mut test, template, &ana, 3);
    join(&mut test, circle, &budi);

    // Cannot contribute before the start.
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_method(ana.account, "withdraw", args![TARI_TOKEN, CONTRIBUTION])
            .put_last_instruction_output_on_workspace("coins")
            .call_method(circle, "contribute", args![Workspace("coins")])
            .build_and_seal(&ana.key),
        vec![],
    );
    assert_reject_reason(reason, "Circle has not started");

    start(&mut test, circle, &ana);

    // An outsider cannot contribute.
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_method(outsider.account, "withdraw", args![TARI_TOKEN, CONTRIBUTION])
            .put_last_instruction_output_on_workspace("coins")
            .call_method(circle, "contribute", args![Workspace("coins")])
            .build_and_seal(&outsider.key),
        vec![],
    );
    assert_reject_reason(reason, "Not a member of this circle");

    // The wrong amount is rejected and nothing is charged: the balance is
    // unchanged after the rejected transaction.
    let before = tari_balance(&test, ana.account);
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_method(ana.account, "withdraw", args![TARI_TOKEN, CONTRIBUTION + 1])
            .put_last_instruction_output_on_workspace("coins")
            .call_method(circle, "contribute", args![Workspace("coins")])
            .build_and_seal(&ana.key),
        vec![],
    );
    assert_reject_reason(reason, "Contribution must be exactly the circle amount");
    assert_eq!(tari_balance(&test, ana.account), before);

    // Paying twice in one round is rejected.
    contribute(&mut test, circle, &ana, CONTRIBUTION);
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_method(ana.account, "withdraw", args![TARI_TOKEN, CONTRIBUTION])
            .put_last_instruction_output_on_workspace("coins")
            .call_method(circle, "contribute", args![Workspace("coins")])
            .build_and_seal(&ana.key),
        vec![],
    );
    assert_reject_reason(reason, "Already paid this round");
}

#[test]
fn payout_requires_everyone_paid() {
    let mut test = TemplateTest::my_crate();
    let template = test.get_template_address(TEMPLATE_NAME);

    let ana = new_member(&mut test);
    let budi = new_member(&mut test);
    let circle = create_circle(&mut test, template, &ana, 2);
    join(&mut test, circle, &budi);
    start(&mut test, circle, &ana);

    contribute(&mut test, circle, &ana, CONTRIBUTION);

    let reason = test.execute_expect_failure(
        test.transaction()
            .call_method(circle, "payout", args![])
            .build_and_seal(&ana.key),
        vec![],
    );
    assert_reject_reason(reason, "Not all members have paid this round");
}

#[test]
fn default_settlement_expels_defaulter_refunds_payers_and_continues() {
    let mut test = TemplateTest::my_crate();
    let template = test.get_template_address(TEMPLATE_NAME);

    let ana = new_member(&mut test);
    let budi = new_member(&mut test);
    let citra = new_member(&mut test); // will stop paying
    let circle = create_circle(&mut test, template, &ana, 3);
    join(&mut test, circle, &budi);
    join(&mut test, circle, &citra);
    start(&mut test, circle, &ana);

    // Round 0: Ana and Budi pay, Citra does not.
    contribute(&mut test, circle, &ana, CONTRIBUTION);
    contribute(&mut test, circle, &budi, CONTRIBUTION);

    // Before the deadline, settling is rejected.
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_method(circle, "settle_default", args![])
            .build_and_seal(&ana.key),
        vec![],
    );
    assert_reject_reason(reason, "Round deadline has not passed yet");

    // Move past the deadline (start was at epoch 0, round length 10).
    test.set_virtual_substate(
        VirtualSubstateId::CurrentEpoch,
        VirtualSubstate::CurrentEpoch(ROUND_LEN),
    );

    let ana_before = tari_balance(&test, ana.account);
    let budi_before = tari_balance(&test, budi.account);
    let result = test.execute_expect_success(
        test.transaction()
            .call_method(circle, "settle_default", args![])
            .build_and_seal(&budi.key),
        vec![],
    );

    // Payers are refunded in full.
    assert_eq!(
        tari_balance(&test, ana.account),
        ana_before + Amount::new(u128::from(CONTRIBUTION))
    );
    assert_eq!(
        tari_balance(&test, budi.account),
        budi_before + Amount::new(u128::from(CONTRIBUTION))
    );
    // Citra is expelled; the circle continues with 2 members.
    assert_eq!(test.call_method::<u32>(circle, "member_count", args![], vec![]), 2);
    assert!(!test.call_method::<bool>(circle, "is_completed", args![], vec![]));
    assert_eq!(
        test.call_method::<Amount>(circle, "pot_balance", args![], vec![]),
        Amount::new(0)
    );
    assert!(result
        .finalize
        .events
        .iter()
        .any(|e| e.topic() == "ArisanCircle.MembersExpelled"));

    // Round 0 restarts with the two remaining members: Ana takes the pot.
    contribute(&mut test, circle, &ana, CONTRIBUTION);
    contribute(&mut test, circle, &budi, CONTRIBUTION);
    let ana_before = tari_balance(&test, ana.account);
    payout(&mut test, circle, &budi);
    assert_eq!(
        tari_balance(&test, ana.account),
        ana_before + Amount::new(200)
    );

    // Round 1: Budi takes the pot, circle completes.
    contribute(&mut test, circle, &ana, CONTRIBUTION);
    contribute(&mut test, circle, &budi, CONTRIBUTION);
    payout(&mut test, circle, &ana);
    assert!(test.call_method::<bool>(circle, "is_completed", args![], vec![]));

    // Citra paid nothing and received nothing; her balance is untouched.
    assert_eq!(
        tari_balance(&test, citra.account),
        Amount::from(INITIAL_BALANCE)
    );
}

#[test]
fn settle_default_rejects_when_everyone_paid() {
    let mut test = TemplateTest::my_crate();
    let template = test.get_template_address(TEMPLATE_NAME);

    let ana = new_member(&mut test);
    let budi = new_member(&mut test);
    let circle = create_circle(&mut test, template, &ana, 2);
    join(&mut test, circle, &budi);
    start(&mut test, circle, &ana);
    contribute(&mut test, circle, &ana, CONTRIBUTION);
    contribute(&mut test, circle, &budi, CONTRIBUTION);

    test.set_virtual_substate(
        VirtualSubstateId::CurrentEpoch,
        VirtualSubstate::CurrentEpoch(ROUND_LEN),
    );
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_method(circle, "settle_default", args![])
            .build_and_seal(&ana.key),
        vec![],
    );
    assert_reject_reason(reason, "Everyone has paid - call payout instead");
}

#[test]
fn circle_dissolves_when_too_few_members_remain() {
    let mut test = TemplateTest::my_crate();
    let template = test.get_template_address(TEMPLATE_NAME);

    let ana = new_member(&mut test);
    let budi = new_member(&mut test); // will stop paying
    let circle = create_circle(&mut test, template, &ana, 2);
    join(&mut test, circle, &budi);
    start(&mut test, circle, &ana);

    contribute(&mut test, circle, &ana, CONTRIBUTION);
    test.set_virtual_substate(
        VirtualSubstateId::CurrentEpoch,
        VirtualSubstate::CurrentEpoch(ROUND_LEN),
    );

    test.execute_expect_success(
        test.transaction()
            .call_method(circle, "settle_default", args![])
            .build_and_seal(&ana.key),
        vec![],
    );

    // One member left: the circle dissolves, Ana is refunded, nobody loses.
    assert!(test.call_method::<bool>(circle, "is_completed", args![], vec![]));
    assert_eq!(
        tari_balance(&test, ana.account),
        Amount::from(INITIAL_BALANCE)
    );
    assert_eq!(
        test.call_method::<u32>(circle, "member_count", args![], vec![]),
        1
    );
}

#[test]
fn constructor_validates_its_arguments() {
    let mut test = TemplateTest::my_crate();
    let template = test.get_template_address(TEMPLATE_NAME);
    let ana = new_member(&mut test);

    // Zero contribution.
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_function(
                template,
                "new",
                args![Amount::new(0), 3u32, ROUND_LEN, ana.account],
            )
            .build_and_seal(&ana.key),
        vec![],
    );
    assert_reject_reason(reason, "Contribution must be greater than zero");

    // Max members below 2.
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_function(
                template,
                "new",
                args![Amount::new(u128::from(CONTRIBUTION)), 1u32, ROUND_LEN, ana.account],
            )
            .build_and_seal(&ana.key),
        vec![],
    );
    assert_reject_reason(reason, "A circle needs at least 2 members");

    // Zero round length.
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_function(
                template,
                "new",
                args![Amount::new(u128::from(CONTRIBUTION)), 3u32, 0u64, ana.account],
            )
            .build_and_seal(&ana.key),
        vec![],
    );
    assert_reject_reason(reason, "Round length must be greater than zero");
}

#[test]
fn early_recipient_who_defaults_cannot_take_anything_more() {
    // The classic arisan attack: the first recipient takes the pot, then stops
    // paying. The template bounds the damage: they are expelled, the payers
    // are refunded, and the defaulter's only gain is the one pot they already
    // received (minus their own contribution) — they can never touch another.
    let mut test = TemplateTest::my_crate();
    let template = test.get_template_address(TEMPLATE_NAME);

    let ana = new_member(&mut test); // round-0 recipient, then defaults
    let budi = new_member(&mut test);
    let citra = new_member(&mut test);
    let circle = create_circle(&mut test, template, &ana, 3);
    join(&mut test, circle, &budi);
    join(&mut test, circle, &citra);
    start(&mut test, circle, &ana);

    // Round 0 completes normally: Ana receives 300 (net +200).
    contribute(&mut test, circle, &ana, CONTRIBUTION);
    contribute(&mut test, circle, &budi, CONTRIBUTION);
    contribute(&mut test, circle, &citra, CONTRIBUTION);
    payout(&mut test, circle, &ana);
    assert_eq!(
        tari_balance(&test, ana.account),
        Amount::from(INITIAL_BALANCE) + Amount::new(200)
    );

    // Round 1: Ana stops paying. Budi and Citra pay, deadline passes, settle.
    contribute(&mut test, circle, &budi, CONTRIBUTION);
    contribute(&mut test, circle, &citra, CONTRIBUTION);
    test.set_virtual_substate(
        VirtualSubstateId::CurrentEpoch,
        VirtualSubstate::CurrentEpoch(2 * ROUND_LEN),
    );
    test.execute_expect_success(
        test.transaction()
            .call_method(circle, "settle_default", args![])
            .build_and_seal(&budi.key),
        vec![],
    );

    // Ana is expelled and gains nothing more; Budi and Citra are refunded.
    assert_eq!(test.call_method::<u32>(circle, "member_count", args![], vec![]), 2);
    assert_eq!(
        tari_balance(&test, ana.account),
        Amount::from(INITIAL_BALANCE) + Amount::new(200)
    );
    assert_eq!(
        tari_balance(&test, budi.account),
        Amount::from(INITIAL_BALANCE) - Amount::new(100)
    );
    assert_eq!(
        tari_balance(&test, citra.account),
        Amount::from(INITIAL_BALANCE) - Amount::new(100)
    );

    // The circle finishes between Budi and Citra: round 0 (shifted back after
    // Ana's expulsion) pays Budi, round 1 pays Citra, then it completes.
    // Each of them is short exactly one contribution (100) versus the
    // no-default outcome — the bounded cost of Ana's default.
    contribute(&mut test, circle, &budi, CONTRIBUTION);
    contribute(&mut test, circle, &citra, CONTRIBUTION);
    let budi_before = tari_balance(&test, budi.account);
    payout(&mut test, circle, &citra);
    assert_eq!(
        tari_balance(&test, budi.account),
        budi_before + Amount::new(200)
    );
    assert_eq!(test.call_method::<u32>(circle, "current_round", args![], vec![]), 1);

    contribute(&mut test, circle, &budi, CONTRIBUTION);
    contribute(&mut test, circle, &citra, CONTRIBUTION);
    let citra_before = tari_balance(&test, citra.account);
    payout(&mut test, circle, &budi);
    assert_eq!(
        tari_balance(&test, citra.account),
        citra_before + Amount::new(200)
    );
    assert!(test.call_method::<bool>(circle, "is_completed", args![], vec![]));

    assert_eq!(
        tari_balance(&test, budi.account),
        Amount::from(INITIAL_BALANCE) - Amount::new(100)
    );
    assert_eq!(
        tari_balance(&test, citra.account),
        Amount::from(INITIAL_BALANCE) - Amount::new(100)
    );
}
