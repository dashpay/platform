const networks = require('@dashevo/dashcore-lib/lib/networks');

const DAPIAddress = require('./DAPIAddress');

class SimplifiedMasternodeListDAPIAddressProvider {
  /**
   * @param {SimplifiedMasternodeListProvider} smlProvider
   * @param {ListDAPIAddressProvider} listDAPIAddressProvider
   * @param {DAPIAddress[]} addressWhiteList
   * @param {DAPIClientOptions} [options]
   */
  constructor(smlProvider, listDAPIAddressProvider, addressWhiteList, options = {}) {
    this.smlProvider = smlProvider;
    this.options = options;
    this.listDAPIAddressProvider = listDAPIAddressProvider;
    this.addressWhiteStrings = addressWhiteList.map((dapiAddress) => dapiAddress.toString());
  }

  /**
   * Get random live DAPI address from SML
   * @returns {Promise<DAPIAddress>}
   */
  async getLiveAddress() {
    const sml = await this.smlProvider.getSimplifiedMNList();
    const validMasternodeList = sml.getValidMasternodesList()
      // Keep only HP masternodes
      .filter((smlEntry) => smlEntry.nType === 1);

    const addressesByRegProTxHashes = {};
    let allowSelfSignedCertificate;
    this.listDAPIAddressProvider.getAllAddresses().forEach((address) => {
      allowSelfSignedCertificate = address.isSelfSignedCertificateAllowed();

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
          port: smlEntry.platformHTTPPort,
          allowSelfSignedCertificate,
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

    const liveAddress = await this.listDAPIAddressProvider.getLiveAddress();

    // This is a temporary fix for a localhost masternode.
    // On macOS, internal docker IP is used to register masternode, and it's
    // not really possible to bind to that address, so that workaround is introduced.
    //
    // It lives here, and only here, because only addresses discovered from the
    // masternode list can hold such an unreachable docker-internal host. An
    // address list the caller supplies (`dapiAddresses`, `seeds`) already names
    // the exact gateway to talk to (dashmate e2e suites move the stock ports on
    // purpose), and clobbering it with the stock local ports silently redirects
    // every request to whichever network squats those ports on the machine.
    const network = networks.get(this.options.network);
    if (liveAddress && network && network.regtestEnabled) {
      const liveAddressCount = this.listDAPIAddressProvider.getLiveAddresses().length;
      const randomNodeIndex = Math.floor(Math.random() * liveAddressCount);

      liveAddress.protocol = 'https';
      liveAddress.host = '127.0.0.1';
      liveAddress.allowSelfSignedCertificate = true;
      liveAddress.port = 2443 + randomNodeIndex * 100;
    }

    return liveAddress;
  }

  /**
   * Check if we have live addresses left
   * @returns {Promise<boolean>}
   */
  async hasLiveAddresses() {
    return this.listDAPIAddressProvider.hasLiveAddresses();
  }
}

module.exports = SimplifiedMasternodeListDAPIAddressProvider;
