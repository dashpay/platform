const find = require('lodash/find');

const utils = {
  // Removes rank from mnLists
  // (rank not needed for Quorum determination and cause mnLists to mutate too frequently)
  getCacheableList(mnList) {
    return mnList.filter(l => delete l.rank);
  },
  getDiff(oldList, newList) {
    return {
      additions: newList.filter(mn => !find(oldList, mn)),
      deletions: oldList.filter(mn => !find(newList, mn)).map(mn => mn.vin),
    };
  },

};

module.exports = utils;
