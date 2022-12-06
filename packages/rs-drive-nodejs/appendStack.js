/**
 * @param {Function} fn
 * @returns {(function(...[*]): Promise<*>)}
 */
function appendStackAsync(fn) {
  return async function appendStackWrapper(...args) {
    try {
      return await fn.call(this, ...args);
    } catch (e) {
      e.stack = (new Error(e.message)).stack;

      throw e;
    }
  };
}

/**
 * @param {Function} fn
 * @returns {(function(...[*]): *)}
 */
function appendStack(fn) {
  return function appendStackWrapper(...args) {
    try {
      return fn.call(this, ...args);
    } catch (e) {
      e.stack = (new Error(e.message)).stack;

      throw e;
    }
  };
}

module.exports = {
  appendStack,
  appendStackAsync,
};
