module aava::viewer;

// === Imports ===

use std::string::String;
use sui::{
    derived_object,
    vec_map::{Self, VecMap},
};
use aava::{
    account_registry::AccountRegistry,
    protocol_authority::ViewerAuth,
};

// === Errors ===

const EInvalidSanctionKind: u64 = 0;

// === Constants ===

const WARNED: u8 = 0;
const REVOKED: u8 = 1;

// === Structs ===

public struct Account has key {
    id: UID,
    // off-chain app user handle (used to derive the account ID)
    handle: String,
    // owner address, none if not claimed
    owner: Option<address>,
    // protocol address, instantiates the viewer account
    protocol: address,
    // sanctions applied to the account
    sanctions: vector<Sanction>,
    // additional metadata
    metadata: VecMap<String, String>,
}

public struct Sanction has copy, store, drop {
    // session for which the sanction has been applied
    session_id: String,
    // stream for which the sanction has been applied
    stream_id: ID,
    // account that issued the sanction
    issuer: ID,
    // sanction type (warned or revoked)
    kind: u8,
    // sanction reason
    reason: String,
    // sanction start date
    timestamp_ms: u64,
}

// === Public functions ===

// TODO: might have a registry per app later (managed by Creator?)

public fun new_account(
    registry: &mut AccountRegistry,
    _: &ViewerAuth,
    handle: String,
    ctx: &mut TxContext,
) {
    let account = Account {
        id: derived_object::claim(registry.uid_mut(), handle),
        handle,
        owner: option::none(),
        protocol: ctx.sender(),
        sanctions: vector::empty(),
        metadata: vec_map::empty(),
    };

    transfer::share_object(account);
}

public(package) fun add_sanction(
    account: &mut Account,
    session_id: String,
    stream_id: ID,
    issuer: ID,
    kind: u8,
    reason: String,
    timestamp_ms: u64,
) {
    assert!(kind == WARNED || kind == REVOKED, EInvalidSanctionKind);

    let sanction = Sanction {
        session_id,
        stream_id,
        issuer,
        kind,
        reason,
        timestamp_ms,
    };

    account.sanctions.push_back(sanction);
}

// === View functions ===

public fun owner(account: &Account): &Option<address> {
    &account.owner
}

public fun protocol(account: &Account): address {
    account.protocol
}