/**
 * @param {*} object
 * @returns {boolean}
 */
function isObject(object) {
  return (typeof object === 'object') && (object !== null);
}

module.exports = isObject;
