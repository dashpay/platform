/**
 * Reject with @param error if @param promise is not resolved in @param ms
 *
 * @param {Promise} promise
 * @param {Error} error
 * @param {number} ms
 * @return {Promise<any>}
 */
module.exports = async function rejectAfter(promise, error, ms) {
  let timeout;
  let res;
  try {
    res = await Promise.race([
      promise,
      new Promise((resolve, reject) => {
        timeout = setTimeout(() => reject(error), ms);
      }),
    ]);
  } finally {
    // noinspection JSUnusedAssignment
    clearTimeout(timeout);
  }

  return res;
};
