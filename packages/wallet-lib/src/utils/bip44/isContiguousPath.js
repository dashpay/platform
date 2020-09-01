const is = require('../is');

module.exports = function isContiguousPath(currPath, prevPath) {
  if (is.undef(currPath)) return false;

  const splitedCurrPath = currPath.split('/');
  const currIndex = parseInt(splitedCurrPath[5], 10);

  if (is.undef(prevPath)) {
    return currIndex === 0;
  }
  const splitedPrevPath = prevPath.split('/');
  const prevIndex = parseInt(splitedPrevPath[5], 10);
  return prevIndex === currIndex - 1;
};
