const { map, filter, difference } = require('lodash');
const { WALLET_TYPES } = require('../CONSTANTS');

function classifyAddresses(addressStore, accountIndex, walletType) {
  const { external, internal, misc } = addressStore;

  // This will filter addresses to return only the one that are directly one the account manage.
  // TODO: Computational improvement can be made by having accountIndex
  //  member of the address info format and thus comparing only 2 numbers
  const filterPathByAccount = (address) => (parseInt(address.path.split('/')[3], 10) === accountIndex);
  const addressMappingPredicate = (addressInfo) => (addressInfo.address);

  const externalAddressList = (walletType === WALLET_TYPES.HDWALLET)
    ? map(filter(external, filterPathByAccount), addressMappingPredicate)
    : map(misc, addressMappingPredicate);

  const internalAddressList = (walletType === WALLET_TYPES.HDWALLET)
    ? map(filter(internal, filterPathByAccount), addressMappingPredicate)
    : [];

  const otherAccountAddressList = (walletType === WALLET_TYPES.HDWALLET)
    ? difference(
      [...map(external, addressMappingPredicate), ...map(internal, addressMappingPredicate)],
      [...externalAddressList, ...internalAddressList],
    )
    : [];

  return {
    externalAddressList,
    internalAddressList,
    otherAccountAddressList,
  };
}
module.exports = classifyAddresses;
