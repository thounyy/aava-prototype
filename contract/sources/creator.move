module aava::creator;

// === Imports ===

use std::string::String;
use sui::{
    derived_object,
    dynamic_object_field as dof,
    coin::Coin,
    vec_map::{Self, VecMap},
    vec_set::{Self, VecSet},
};
use wal::wal::WAL;
use walrus::system::System;
use enclave::enclave::Enclave;
use aava::{
    account_registry::AccountRegistry,
    protocol_authority::CreatorRequest,
    blob_id::{Self, BLOB_ID},
};

// === Aliases ===

use fun dof::add as UID.add;
use fun dof::borrow_mut as UID.borrow_mut;

// === Errors ===

const ENotMember: u64 = 0;
const EStreamNotActive: u64 = 1;

// === Structs ===

/// Parent struct protecting the config.
public struct Account has key {
    id: UID,
    // addresses of the creators, there can by multiple owners
    members: VecSet<address>,
    // additional metadata
    metadata: VecMap<String, String>,
}

public struct StreamKey(ID) has copy, drop, store;

public struct Stream has key, store {
    id: UID,
    // active, stored, invalid
    status: String,
    // df(Blob) if none, sessions dataset is not stored on walrus
}

public struct BlobKey() has copy, drop, store;

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

public fun start_stream(
    account: &mut Account,
    ctx: &mut TxContext,
) {
    assert!(is_member(account, ctx.sender()), ENotMember);

    let id = object::new(ctx);
    let key = StreamKey(id.to_inner());
    let stream = Stream {
        id,
        status: "active",
    };
    
    account.id.add(key, stream);
}

public fun end_stream(
    account: &mut Account,
    enclave: &Enclave<BLOB_ID>,
    system: &mut System,
    payment: &mut Coin<WAL>,
    stream_id: ID,
    timestamp_ms: u64,
    sig: &vector<u8>,
    blob_id: u256,
    root_hash: u256,
    unencoded_size: u64,
    encoding_type: u8,
    encoded_size: u64,
    deletable: bool,
    ctx: &mut TxContext,
) {
    assert!(account.is_member(ctx.sender()), ENotMember);

    let key = StreamKey(stream_id);
    let stream: &mut Stream = account.id.borrow_mut(key);
    assert!(stream.status == "active", EStreamNotActive);

    // verify the blob_id bytes from the enclave
    if (blob_id::verify(enclave, blob_id, timestamp_ms, sig)) {
        // reserve storage space
        let storage = system.reserve_space(
            encoded_size, 
            53, 
            payment,
            ctx
        );

        // register the blob
        let blob = system.register_blob(
            storage,
            blob_id,
            root_hash,
            unencoded_size,
            encoding_type,
            deletable,
            payment,
            ctx,
        );
        
        stream.id.add(BlobKey(), blob);
        stream.status = "stored";
    } else {
        stream.status = "invalid";
    }
}

// === View functions ===

public fun members(account: &Account): VecSet<address> {
    account.members
}

public fun is_member(account: &Account, addr: address): bool {
    account.members.contains(&addr)
}

// === Member functions ===

public fun add_member(
    account: &mut Account,
    addr: address,
) {
    assert!(is_member(account, addr), ENotMember);
    account.members.insert(addr);
}

public fun remove_member(
    account: &mut Account,
    addr: address,
) {
    assert!(is_member(account, addr), ENotMember);
    account.members.remove(&addr);
}