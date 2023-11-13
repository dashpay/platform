/**
 *
 * @param dmnState {dmnState}
 * @param enabledMasternodes {number}
 * @param enabledEvonodes {number}
 * @param coreBlocks {number} current block height
 * @return {number}
 */
export function calculatePaymentQueuePosition(dmnState, enabledMasternodes, enabledEvonodes, coreBlocks) {
  const enabledCount = enabledMasternodes + enabledEvonodes * 4;

  let paymentQueuePosition;
  // Masternode has been unbanned recently
  if (dmnState.PoSeRevivedHeight > dmnState.lastPaidHeight) {
    paymentQueuePosition = dmnState.PoSeRevivedHeight
      + enabledCount
      - coreBlocks;
  // Masternode has never been paid
  } else if (dmnState.lastPaidHeight === 0) {
    paymentQueuePosition = dmnState.registeredHeight
      + enabledCount
      - coreBlocks;
  // Masternode was previously paid and is in normal queue
  } else {
    paymentQueuePosition = dmnState.lastPaidHeight
      + enabledCount
      - coreBlocks;
  }
  return paymentQueuePosition;
}
