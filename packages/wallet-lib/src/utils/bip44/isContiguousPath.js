const is = require('../is');

module.exports = function isContiguousPath(currPath, prevPath) {
  if (is.undef(currPath)) return false;

  const splitCurrPath = currPath.split('/');
  const currIndex = parseInt(splitCurrPath[5], 10);

  if (is.undef(prevPath)) {
    return currIndex === 0;
  }
  const splitPrevPath = prevPath.split('/');
  const prevIndex = parseInt(splitPrevPath[5], 10);
  return prevIndex === currIndex - 1;
};
