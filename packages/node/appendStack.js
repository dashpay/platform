/**
 * @param {Function} fn
 * @returns {(function(...[*]): Promise<*>)}
 */
function appendStack(fn) {
  return async function appendStackWrapper(...args) {
    try {
      return await fn.call(this, ...args);
    } catch (e) {
      e.stack = (new Error(e.message)).stack;

      throw e;
    }
  };
}

module.exports = appendStack;
