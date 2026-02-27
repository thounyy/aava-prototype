#[macro_export]
macro_rules! build_and_execute_tx {
    ($client:expr, $pk:expr, |$builder:ident| $build_block:block) => {{
        async {
            let mut $builder = ::sui_transaction_builder::TransactionBuilder::new();
            $builder.set_sender($pk.public_key().derive_address());
            $build_block

            let tx = $builder.build($client).await.map_err(|err| {
                ::tracing::warn!("Failed to build Sui transaction: {err}");
                ::axum::http::StatusCode::INTERNAL_SERVER_ERROR
            })?;

            let signature = ::sui_crypto::SuiSigner::sign_transaction($pk, &tx).map_err(|err| {
                ::tracing::warn!("Failed to sign Sui transaction: {err}");
                ::axum::http::StatusCode::INTERNAL_SERVER_ERROR
            })?;

            let response = $client
                .execute_transaction_and_wait_for_checkpoint(
                    ::sui_rpc::proto::sui::rpc::v2::ExecuteTransactionRequest::new(tx.into())
                        .with_signatures(vec![signature.into()]),
                    ::std::time::Duration::from_secs(10),
                )
                .await
                .map_err(|err| {
                    ::tracing::warn!("Failed to execute Sui transaction: {err}");
                    ::axum::http::StatusCode::INTERNAL_SERVER_ERROR
                })?
                .into_inner();

            Ok::<_, ::axum::http::StatusCode>(response)
        }
        .await
    }};
}