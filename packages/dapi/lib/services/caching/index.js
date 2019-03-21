// TODO: Address ESLint issues the next time this file is edited
/* eslint-disable */
const nodeCache = require('node-cache');

const cache = new nodeCache();
const ttl = 60 * 60 * 600; // 1 hour
const listUtils = require('../../utils/listUtils');

class CacheContoller {
  constructor() {
    this.diffCache = []; // myCache not used because we need an ordered queue structure to retrieve updates
  }

  set(key, value) {
    cache.set(key, value, ttl);
  }

  get(key) {
    return new Promise(((resolve, reject) => {
      cache.get(key, (err, res) => {
        if (!err) {
          resolve(res);
        } else {
          reject(`Cache fetch error: ${err}`);
        }
      });
    }));
  }

  setMnList(hash, list) {
    if (this.lastMnListKey) {
      this.cacheDifferenceSet(list);
    }
    this.lastMnListKey = hash;
    this.set(hash, list);
  }

  getMnList(hash) {
    return this.get(hash);
  }

  getLastMnList() {
    return this.get(this.lastMnListKey);
  }

  getLastMnListKey() {
    return this.lastMnListKey;
  }

  cacheDifferenceSet(newList) {
    const self = this;
    Promise.all([this.get(this.lastMnListKey), Promise.resolve(this.lastMnListKey)])
      .then(([oldList, oldHash]) => {
        self.diffCache.push({
          hash: oldHash,
          diff: listUtils.getDiff(oldList, newList),
        });
      });
  }

  isDiffCached(hash) {
    return this.diffCache && this.diffCache.filter(i => i.hash == hash).length == 1;
  }

  getDiffCache(hash) {
    const index = this.diffCache.findIndex(i => i.hash == hash);
    const fullUpdate = this.diffCache.slice(index);
    return {
      additions: [].concat.apply([], fullUpdate.map(u => u.diff.additions)),
      deletions: [].concat.apply([], fullUpdate.map(u => u.diff.deletions)),
    };
  }
}

module.exports = CacheContoller;
