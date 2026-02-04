module aava::account_registry;

// === Structs ===

public struct AccountRegistry has key {
    id: UID,
}

// === Public functions ===

fun init(ctx: &mut TxContext) {
    transfer::share_object(AccountRegistry { id: object::new(ctx) });
}

public(package) fun uid_mut(account_registry: &mut AccountRegistry): &mut UID {
    &mut account_registry.id
}
