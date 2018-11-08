/**
 * @param array
 * @return {array}
 */
module.exports = function findDuplicates(array) {
  const count = arr => arr.reduce((a, b) => Object.assign(a, { [b]: (a[b] || 0) + 1 }), {});
  const duplicates = dict => Object.keys(dict).filter(a => dict[a] > 1);
  return duplicates(count(array));
};
