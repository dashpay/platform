function getPaymentQueuePosition(dmnState, masternodeEnabledCount, coreBlocks) {
  let paymentQueuePosition;
  // Masternode has been unbanned recently
  if (dmnState.PoSeRevivedHeight > dmnState.lastPaidHeight) {
    paymentQueuePosition = dmnState.PoSeRevivedHeight
      + masternodeEnabledCount
      - coreBlocks;
  // Masternode has never been paid
  } else if (dmnState.lastPaidHeight === 0) {
    paymentQueuePosition = dmnState.registeredHeight
      + masternodeEnabledCount
      - coreBlocks;
  // Masternode was previously paid and is in normal queue
  } else {
    paymentQueuePosition = dmnState.lastPaidHeight
      + masternodeEnabledCount
      - coreBlocks;
  }
  return paymentQueuePosition;
}

module.exports = getPaymentQueuePosition;
