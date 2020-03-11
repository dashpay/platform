const { is } = require('../../../../utils');

module.exports = async function getAddressSummary(address) {
  if (!is.address(address)) throw new Error('Received an invalid address ot fetch');
  return this.client.getAddressSummary(address);
};
