/**
 *
 * @param {object} obj
 * @param {string} methodName
 * @return {boolean}
 */
function hasMethod(obj, methodName) {
  return !!obj && typeof obj[methodName] === 'function';
}
export default hasMethod;