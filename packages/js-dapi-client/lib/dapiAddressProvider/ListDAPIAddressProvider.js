const sample = (arr) => arr[Math.floor(Math.random() * arr.length)];
const networks = require('@dashevo/dashcore-lib/lib/networks');

class ListDAPIAddressProvider {
  /**
   * @param {DAPIAddress[]} addresses
   * @param {DAPIClientOptions} [options]
   */
  constructor(addresses, options = {}) {
    this.options = {
      baseBanTime: 60 * 1000,
      ...options,
    };

    this.addresses = addresses;
  }

  /**
   * Get random address
   * @returns {Promise<DAPIAddress|undefined>}
   */
  async getLiveAddress() {
    const liveAddresses = this.getLiveAddresses();

    const liveAddress = sample(liveAddresses);

    if (liveAddress === undefined) {
      return undefined;
    }

    // This is a temporary fix for a localhost masternode.
    // On macOS, internal docker IP is used to register masternode, and it's
    // not really possible to bind to that address, so that workaround is introduced.
    //
    // Only addresses discovered from the masternode list (they carry the
    // masternode's proRegTxHash) can hold such an unreachable docker-internal
    // host, so only those are rewritten, and only when the host is not
    // already a reachable loopback. A caller-supplied address — a moved-port
    // loopback, a secondary loopback like 127.0.0.2, a LAN IP, or a container
    // hostname — already names the exact gateway to talk to (dashmate e2e
    // suites move the stock ports on purpose), and clobbering it with the
    // stock local ports silently redirects every request to whichever network
    // squats those ports on the machine.
    const network = networks.get(this.options.network);
    const isLoopback = ['127.0.0.1', 'localhost'].includes(liveAddress.getHost());
    const isFromMasternodeList = Boolean(liveAddress.getProRegTxHash());
    if (network && network.regtestEnabled && isFromMasternodeList && !isLoopback) {
      const randomNodeIndex = Math.floor(Math.random() * liveAddresses.length);

      liveAddress.protocol = 'https';
      liveAddress.host = '127.0.0.1';
      liveAddress.allowSelfSignedCertificate = true;
      liveAddress.port = 2443 + randomNodeIndex * 100;
    }

    return liveAddress;
  }

  /**
   * Get all addresses
   * @returns {DAPIAddress[]}
   */
  getAllAddresses() {
    return this.addresses;
  }

  /**
   * Set addresses
   * @param {DAPIAddress[]} addresses
   * @returns {ListDAPIAddressProvider}
   */
  setAddresses(addresses) {
    this.addresses = addresses;

    return this;
  }

  /**
   * Check if we have live addresses left
   * @returns {Promise<boolean>} - True if there are live address left
   */
  async hasLiveAddresses() {
    const liveAddresses = this.getLiveAddresses();

    return liveAddresses.length > 0;
  }

  /**
   * Get live addresses
   * @returns {DAPIAddress[]}
   */
  getLiveAddresses() {
    const now = Date.now();

    return this.addresses.filter((address) => {
      if (!address.isBanned()) {
        return true;
      }

      // Exponentially increase ban time based on ban count
      const coefficient = Math.exp(address.getBanCount() - 1);
      const banPeriod = Math.floor(coefficient) * this.options.baseBanTime;

      return now > address.getBanStartTime() + banPeriod;
    });
  }
}

module.exports = ListDAPIAddressProvider;
