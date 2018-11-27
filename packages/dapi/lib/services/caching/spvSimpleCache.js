class SimpleCache {
  constructor() {
    this.cache = {};
  }

  set(filterHash, updateObj) {
    // Init
    this.cache[filterHash] = this.cache[filterHash]
      || {
        transactions: [],
        merkleblocks: [],
      };

    if (updateObj.constructor.name === 'Transaction') {
      this.cache[filterHash].transactions.push(updateObj);
    } else {
      this.cache[filterHash].merkleblocks.push(updateObj);
    }
  }

  get(filterHash) {
    return this.cache[filterHash];
  }

  getAllFilterHashes() {
    return Object.keys(this.cache);
  }

  clear(filterHash) {
    delete this.cache[filterHash];
  }

  clearInactiveClients(activeClientsFilterHashes) {
    this.getAllFilterHashes().filter(k => !activeClientsFilterHashes.includes(k))
      .forEach((hash) => {
        this.clear(hash);
      });
  }
}

module.exports = SimpleCache;
