module aava::creator;

// === Imports ===

use std::string::String;
use sui::{
    derived_object,
    vec_map::{Self, VecMap},
    vec_set::{Self, VecSet},
};
use aava::{
    account_registry::AccountRegistry,
    protocol_authority::CreatorRequest,
};


// === Errors ===

const ENotMember: u64 = 0;

// === Structs ===

/// Parent struct protecting the config.
public struct Account has key {
    id: UID,
    // addresses of the creators, there can by multiple owners
    members: VecSet<address>,
    // additional metadata
    metadata: VecMap<String, String>,
}

// === Public functions ===

public fun new_account(
    registry: &mut AccountRegistry,
    request: CreatorRequest,
) {
    let (addr, username) = request.complete();

    let account = Account {
        id: derived_object::claim(registry.uid_mut(), username),
        members: vec_set::from_keys(vector[addr]),
        metadata: vec_map::from_keys_values(
            vector["username"], // TODO: add more metadata if necessary
            vector[username]
        ),
    };

    transfer::share_object(account);
}

// === View functions ===

public fun members(account: &Account): VecSet<address> {
    account.members
}

public fun is_member(account: &Account, addr: address): bool {
    account.members.contains(&addr)
}

public fun assert_is_member(account: &Account, ctx: &TxContext) {
    assert!(is_member(account, ctx.sender()), ENotMember);
}