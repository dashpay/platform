function getPaymentQueuePosition(masternodeState, masternodeEnabledCount, coreBlocks) {
  let paymentQueuePosition;
  if (masternodeState.PoSeRevivedHeight > coreBlocks) {
    paymentQueuePosition = masternodeState.PoSeRevivedHeight
      + masternodeEnabledCount
      - coreBlocks;
  } else if (masternodeState.lastPaidHeight === 0) {
    paymentQueuePosition = masternodeState.registeredHeight
      + masternodeEnabledCount
      - coreBlocks;
  } else {
    paymentQueuePosition = masternodeState.lastPaidHeight
      + masternodeEnabledCount
      - coreBlocks;
  }
  return paymentQueuePosition;
}

module.exports = getPaymentQueuePosition;
