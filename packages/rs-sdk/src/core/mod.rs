mod transaction;

//
// async fn wait_for_instant_send_lock_messages(
//     mut stream: rs_dapi_client::tonic::Streaming<TransactionsWithProofsResponse>,
// ) -> Result<InstantSendLockMessages, RegisterIdentityError> {
//     let instant_send_lock_messages;
//     loop {
//         if let Some(TransactionsWithProofsResponse { responses }) = stream
//             .message()
//             .await
//             .map_err(|e| RegisterIdentityError(e.to_string()))?
//         {
//             match responses {
//                 Some(transactions_with_proofs_response::Responses::InstantSendLockMessages(
//                          messages,
//                      )) => {
//                     instant_send_lock_messages = messages;
//                     break;
//                 }
//                 _ => continue,
//             }
//         } else {
//             return Err(RegisterIdentityError(
//                 "steam closed unexpectedly".to_owned(),
//             ));
//         }
//     }
//
//     Ok(instant_send_lock_messages)
// }
