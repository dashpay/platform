/**
 * Reject with @param error if @param promise is not resolved in @param ms
 *
 * @param {Promise} promise
 * @param {Error} error
 * @param {number} ms
 * @return {Promise<any>}
 */
module.exports = function rejectAfter(promise, error, ms) {
  return Promise.race([
    promise,
    new Promise((resolve, reject) => setTimeout(() => reject(error), ms)),
  ]);
};
