const is = require('../../../../utils/is');

module.exports = function getMissingIndexes(paths, fromOrigin = true) {
  if (!is.arr(paths)) return false;

  let sortedIndexes = [];

  paths.forEach((path) => {
    const splitedPath = path.split('/');
    const index = parseInt(splitedPath[5], 10);
    sortedIndexes.push(index);
  });

  sortedIndexes = sortedIndexes.sort((a, b) => a - b);

  let missingIndex = sortedIndexes.reduce((acc, cur, ind, arr) => {
    const diff = cur - arr[ind - 1];
    if (diff > 1) {
      let i = 1;
      while (i < diff) {
        acc.push(arr[ind - 1] + i);
        i += 1;
      }
    }
    return acc;
  }, []);

  // Will fix missing index before our first known indexes
  if (fromOrigin) {
    if (sortedIndexes[0] > 0) {
      for (let i = sortedIndexes[0] - 1; i >= 0; i -= 1) {
        missingIndex.push(i);
      }
    }
  }

  missingIndex = missingIndex.sort((a, b) => a - b);
  return missingIndex;
};
