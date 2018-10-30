/**
 * @param {function} func
 * @return {throttle}
 */
module.exports = function throttleFactory(func) {
  let isInProgress = false;
  let wasCalledDuringProgress = false;

  /**
   * @typedef throttle
   * @return {Promise<void>}
   */
  async function throttle() {
    try {
      if (isInProgress) {
        wasCalledDuringProgress = true;

        return;
      }

      isInProgress = true;

      await func();

      isInProgress = false;

      if (wasCalledDuringProgress) {
        wasCalledDuringProgress = false;
        await throttle();
      }
    } catch (error) {
      isInProgress = false;

      throw error;
    }
  }

  return throttle;
};
