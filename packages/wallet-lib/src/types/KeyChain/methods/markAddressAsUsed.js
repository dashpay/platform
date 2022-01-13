const logger = require('../../../logger');

function markAddressAsUsed(address) {

  const searchResult = [...this.issuedPaths.entries()]
    .find(([, el]) => el.address.toString() === address.toString());

  if (searchResult) {

    const [, addressData] = searchResult;
    logger.silly(`KeyChain - Marking ${address} ${addressData.path} as used`);
    addressData.isUsed = true;

    return this.maybeLookAhead();
  }

}
module.exports = markAddressAsUsed;
