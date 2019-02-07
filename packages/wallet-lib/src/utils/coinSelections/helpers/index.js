const sortAndVerifyUTXOS = require('./sortAndVerifyUTXOS');

module.exports = {
  sortAndVerifyUTXOS,
};

// const is = require('../utils/is');
// const { getBytesOf } = require('../utils/utils');
// const { FEES } = require('../Constants');
// const STRATEGIES = require('./coinSelections/strategies');
//
// /**
//  * Calculate size and value of a provided output
//  * @param outputsList
//  * @return {{outputBytes: number, outputValue: number}}
//  */
// const getOutputsInfo = (outputsList) => {
//   let outputBytes = 0;
//   let outputValue = 0;
//   outputsList.forEach((output) => {
//     outputBytes += getBytesOf(output, 'output');
//     outputValue += output.satoshis;
//   });
//   return {
//     outputBytes,
//     outputValue
//   }
// };
// const estimateFee = (type, txSizeInBytes) => {
//   const satPerKb = FEES[type.toUpperCase()];
//   const txSizeInKB = txSizeInBytes / 1000;
//   const feeInSatoshis = satPerKb * txSizeInBytes;
//
//   return parseInt(feeInSatoshis, 10);
// };
