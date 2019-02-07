const {
  ValidTransportLayerRequired,
} = require('../errors/index');
const { is, dashToDuffs } = require('../utils');

/**
 * Fetch a specific txid from the transport layer
 * @param transactionid - The transaction id to fetch
 * @return {Promise<{txid, blockhash, blockheight, blocktime, fees, size, vout, vin, txlock}>}
 */
async function fetchTransactionInfo(transactionid) {
  if (!this.transport.isValid) throw new ValidTransportLayerRequired('fetchTransactionInfo');

  // valueIn, valueOut,
  const {
    txid, blockhash, blockheight, blocktime, fees, size, vin, vout, txlock,
  } = await this.transport.getTransaction(transactionid);

  const feesInSat = is.float(fees) ? dashToDuffs(fees) : (fees);
  return {
    txid,
    blockhash,
    blockheight,
    blocktime,
    fees: feesInSat,
    size,
    vout,
    vin,
    txlock,
  };
}
module.exports = fetchTransactionInfo;
