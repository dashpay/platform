const dgw = require('dark-gravity-wave-js').isValidTarget;
const utils = require('./utils');

module.exports = {

  isValidBlockHeader(dgwHeaders, newHeader) {
    return newHeader.validProofOfWork() &&
      newHeader.validTimestamp() &&
      dgw.isValidTarget(newHeader.bits, dgwHeaders.map(h => utils.getDgwBlock(h)));
  },
};
