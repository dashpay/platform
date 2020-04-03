/* istanbul ignore next */
async function wait(timeout) {
  const timeStarted = Date.now();
  let finished = false;

  while (!finished) {
    if (Date.now() > timeStarted + timeout) {
      finished = true;
    }
    await Promise.resolve();
  }
}

module.exports = wait;
