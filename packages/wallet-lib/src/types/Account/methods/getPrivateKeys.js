/**
 * Return all the private keys matching the PubKey Addr List
 * @param {[string]} addressList
 * @return {Array}<HDPrivateKey>
 */
function getPrivateKeys(addressList) {
  let addresses = [];
  const privKeys = [];
  if (addressList.constructor.name === Object.name) {
    addresses = [addressList];
  } else { addresses = addressList; }

  const { keyChainStore } = this;

  const keyChain = keyChainStore.getMasterKeyChain();

  addresses.forEach((address) => {
    const addressData = keyChain.getForAddress(address);
    if (addressData) {
      privKeys.push(addressData.key);
    }
  });

  return privKeys;
}
module.exports = getPrivateKeys;
