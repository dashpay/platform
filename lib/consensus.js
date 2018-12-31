const { isValidTarget } = require('@dashevo/dark-gravity-wave');
const utils = require('./utils');

module.exports = {

  isValidBlockHeader(dgwHeaders, newHeader) {
    return newHeader.validProofOfWork() && newHeader.validTimestamp()
      && isValidTarget(
        newHeader.bits,
        dgwHeaders.map(h => utils.getDgwBlock(h)),
      );
  },
};
