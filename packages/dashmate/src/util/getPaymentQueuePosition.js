function getPaymentQueuePosition(masternodeState, masternodeEnabledCount, coreBlocks) {
  let paymentQueuePosition;
  // Masternode has been unbanned recently
  if (masternodeState.PoSeRevivedHeight > masternodeState.lastPaidHeight) {
    paymentQueuePosition = masternodeState.PoSeRevivedHeight
      + masternodeEnabledCount
      - coreBlocks;
  // Masternode has never been paid
  } else if (masternodeState.lastPaidHeight === 0) {
    paymentQueuePosition = masternodeState.registeredHeight
      + masternodeEnabledCount
      - coreBlocks;
  // Masternode was previously paid and is in normal queue
  } else {
    paymentQueuePosition = masternodeState.lastPaidHeight
      + masternodeEnabledCount
      - coreBlocks;
  }
  return paymentQueuePosition;
}

module.exports = getPaymentQueuePosition;
