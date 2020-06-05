const sample = require('lodash.sample');

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
   *
   * @returns {Promise<DAPIAddress>}
   */
  async getLiveAddress() {
    const liveAddresses = this.getLiveAddresses();

    return sample(liveAddresses);
  }

  /**
   * Get all addresses
   *
   * @returns {DAPIAddress[]}
   */
  getAllAddresses() {
    return this.addresses;
  }

  /**
   * Set addresses
   *
   * @param {DAPIAddress[]} addresses
   * @returns {ListDAPIAddressProvider}
   */
  setAddresses(addresses) {
    this.addresses = addresses;

    return this;
  }

  /**
   * Check if we have live addresses left
   *
   * @returns {Promise<boolean>} - True if there are live address left
   */
  async hasLiveAddresses() {
    const liveAddresses = this.getLiveAddresses();

    return liveAddresses.length > 0;
  }

  /**
   * Get live addresses
   *
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
