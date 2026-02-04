module aava::protocol_authority;

// === Imports ===

use std::string::String;
use sui::transfer::Receiving;

// === Errors ===

const ECallerIsNotAuthorized: u64 = 0;

// === Structs ===

// Admin cap to update the protocol address
public struct AdminCap has key {
    id: UID,
}

// Protocol authority to manage the protocol address
public struct ProtocolAuthority has key {
    id: UID,
    // protocol address
    protocol: address,
}

// Hot potato so viewer account creations can be batched and queued
public struct ViewerAuth()

// Hot potato request with authorized address for creator account creation
public struct CreatorRequest has key {
    id: UID,
    // caller address authorized to create the account
    addr: address,
    // authorized username for the account
    username: String,
}

// === Public functions ===

fun init(ctx: &mut TxContext) {
    transfer::share_object(
        ProtocolAuthority { id: object::new(ctx), protocol: ctx.sender() }
    );
    transfer::transfer(
        AdminCap { id: object::new(ctx) }, 
        ctx.sender()
    );
}

// === Viewer functions ===

public fun init_viewers(
    authority: &ProtocolAuthority,
    ctx: &TxContext,
): ViewerAuth {
    assert!(authority.protocol == ctx.sender(), ECallerIsNotAuthorized);
    ViewerAuth()
}

public fun finalize_viewers(auth: ViewerAuth) {
    let ViewerAuth() = auth;
}

// === Creator functions ===

public fun request_creator_account(
    username: String,
    ctx: &mut TxContext,
): CreatorRequest {
    CreatorRequest { 
        id: object::new(ctx), 
        addr: ctx.sender(), 
        username: username,
    }
}

public use fun complete_creator_request as CreatorRequest.complete;
public fun complete_creator_request(
    request: CreatorRequest,
): (address, String) {
    let CreatorRequest { id, addr, username } = request;
    id.delete();

    (addr, username)
}

// for now sends the request to the protocol authority in ptb 
// and receives the request from the protocol
// might change later  

public fun receive_creator_request(
    authority: &mut ProtocolAuthority,
    to_receive: Receiving<CreatorRequest>,
    ctx: &TxContext,
): CreatorRequest {
    assert!(authority.protocol == ctx.sender(), ECallerIsNotAuthorized);
    transfer::receive(&mut authority.id, to_receive)
}

public fun validate_creator_request(
    request: CreatorRequest,
) {
    let addr = request.addr;
    transfer::transfer(request, addr);
}

public fun reject_creator_request(
    request: CreatorRequest,
) {
    let CreatorRequest { id, .. } = request;
    id.delete();
}

// === Authority functions ===

public fun update_protocol(
    authority: &mut ProtocolAuthority,
    _: &AdminCap,
    addr: address,
) {
    authority.protocol = addr;
}