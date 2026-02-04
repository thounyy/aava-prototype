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

// === Structs ===

public struct Account has key {
    id: UID,
    // owner address, none if not claimed
    addr: Option<address>,
    // protocol address, instantiates the viewer account
    protocol: address,
    // additional metadata
    metadata: VecMap<String, String>,
}

// === Public functions ===

public fun new_account(
    registry: &mut AccountRegistry,
    _: &ViewerAuth,
    username: String,
    ctx: &mut TxContext,
) {
    let account = Account {
        id: derived_object::claim(registry.uid_mut(), username),
        addr: option::none(),
        protocol: ctx.sender(),
        metadata: vec_map::from_keys_values(
            vector["username"], // TODO: add more metadata if necessary
            vector[username]
        ),
    };

    transfer::share_object(account);
}

// === View functions ===

public fun addr(account: &Account): &Option<address> {
    &account.addr
}

public fun protocol(account: &Account): address {
    account.protocol
}