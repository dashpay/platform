const dgwTarget = require('dark-gravity-wave-js').getTarget;
const utils = require('./utils');

module.exports = {

  isValidBlockHeader(dgwHeaders, newHeader) {
    return newHeader.validProofOfWork() &&
      newHeader.validTimestamp() &&
      newHeader.bits <= dgwTarget(dgwHeaders.map(h => utils.getDgwBlock(h)));
  },

};
