/**
 * Checks if argument is actually an object
 *
 * @param {*} object
 *
 * @return {boolean}
 */
function isObject(object) {
  return (typeof object === 'object') && (object !== null);
}

module.exports = isObject;
