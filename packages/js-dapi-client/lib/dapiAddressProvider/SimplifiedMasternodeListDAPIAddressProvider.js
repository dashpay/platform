const DAPIAddress = require('./DAPIAddress');

class SimplifiedMasternodeListDAPIAddressProvider {
  /**
   * @param {SimplifiedMasternodeListProvider} smlProvider
   * @param {ListDAPIAddressProvider} listDAPIAddressProvider
   * @param {DAPIAddress[]} addressWhiteList
   */
  constructor(smlProvider, listDAPIAddressProvider, addressWhiteList) {
    this.smlProvider = smlProvider;
    this.listDAPIAddressProvider = listDAPIAddressProvider;
    this.addressWhiteStrings = addressWhiteList.map((dapiAddress) => dapiAddress.toString());
  }

  /**
   * Get random live DAPI address from SML
   *
   * @returns {Promise<DAPIAddress>}
   */
  async getLiveAddress() {
    const sml = await this.smlProvider.getSimplifiedMNList();
    const validMasternodeList = sml.getValidMasternodesList();

    const addressesByRegProTxHashes = {};
    this.listDAPIAddressProvider.getAllAddresses().forEach((address) => {
      if (!address.getProRegTxHash()) {
        return;
      }

      addressesByRegProTxHashes[address.getProRegTxHash()] = address;
    });

    const updatedAddresses = validMasternodeList.map((smlEntry) => {
      let address = addressesByRegProTxHashes[smlEntry.proRegTxHash];

      if (!address) {
        address = new DAPIAddress({
          host: smlEntry.getIp(),
          proRegTxHash: smlEntry.proRegTxHash,
        });
      } else {
        address.setHost(smlEntry.getIp());
      }

      return address;
    });

    let filteredAddresses = updatedAddresses;
    if (this.addressWhiteStrings.length > 0) {
      filteredAddresses = updatedAddresses.filter((dapiAddress) => (
        this.addressWhiteStrings.includes(dapiAddress.toString())
      ));
    }

    this.listDAPIAddressProvider.setAddresses(filteredAddresses);

    return this.listDAPIAddressProvider.getLiveAddress();
  }

  /**
   * Check if we have live addresses left
   *
   * @returns {Promise<boolean>}
   */
  async hasLiveAddresses() {
    return this.listDAPIAddressProvider.hasLiveAddresses();
  }
}

module.exports = SimplifiedMasternodeListDAPIAddressProvider;
