const SimplifiedMNListStore = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListStore');

class SimplifiedMasternodeList {
  constructor(options) {
    this.options = {
      maxListsLimit: options.smlMaxListsLimit,
    };

    this.store = undefined;
  }

  /**
   * @param {SimplifiedMNListDiff[]} smlDiffs
   *
   * @return SimplifiedMasternodeList
   */
  applyDiffs(smlDiffs) {
    if (!this.store) {
      this.store = new SimplifiedMNListStore([...smlDiffs], this.options);
    } else {
      smlDiffs.forEach((diff) => {
        this.store.addDiff(diff);
      });
    }

    return this;
  }

  /**
   *
   * @return {SimplifiedMNListStore|undefined}
   */
  getStore() {
    return this.store;
  }
}

module.exports = SimplifiedMasternodeList;
