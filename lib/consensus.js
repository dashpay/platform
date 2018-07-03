const { isValidTarget } = require('dark-gravity-wave-js');
const utils = require('./utils');

module.exports = {

  isValidBlockHeader(dgwHeaders, newHeader) {
    return newHeader.validProofOfWork() &&
      newHeader.validTimestamp() &&
      isValidTarget(newHeader.bits, dgwHeaders.map(h => utils.getDgwBlock(h)));
  },
};
