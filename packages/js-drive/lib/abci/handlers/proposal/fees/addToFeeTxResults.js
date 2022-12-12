/**
 * @param {{
 *    storageFee: number,
 *    processingFee: number,
 *    feeRefunds: Object<string, number>,
 *    feeRefundsSum: number,
 * }} feeResults
 * @param {{
 *    storageFee: number,
 *    processingFee: number,
 *    feeRefunds: Object<string, number>,
 *    feeRefundsSum: number,
 * }} feeResult
 */
function addToFeeTxResults(feeResults, feeResult) {
  /* eslint-disable no-param-reassign */
  feeResults.storageFee += feeResult.storageFee;
  feeResults.processingFee += feeResult.processingFee;
  feeResults.feeRefundsSum += feeResult.feeRefundsSum;

  for (const [epochIndex, credits] of feeResult.feeRefunds.entries()) {
    if (!feeResults.feeRefunds[epochIndex]) {
      feeResults.feeRefunds[epochIndex] = 0;
    }

    feeResults.feeRefunds[epochIndex] += credits;
  }

  return feeResults;
}

module.exports = addToFeeTxResults;
