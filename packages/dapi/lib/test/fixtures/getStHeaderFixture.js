const { Transaction } = require('@dashevo/dashcore-lib');

const ZERO_HASH = '00'.repeat(32);

function getStHeaderFixture(stPacket) {
  const stHeader = new Transaction();
  stHeader.setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);
  stHeader.extraPayload
    .setCreditFee(1)
    .setRegTxId(ZERO_HASH)
    .setHashPrevSubTx(ZERO_HASH)
    .setHashSTPacket(stPacket.hash());

  return stHeader;
}

module.exports = getStHeaderFixture;
