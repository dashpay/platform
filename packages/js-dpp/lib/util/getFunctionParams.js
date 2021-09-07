const STRIP_COMMENTS = /(\/\/.*$)|(\/\*[\s\S]*?\*\/)|(\s*=[^,)]*(('(?:\\'|[^'\r\n])*')|("(?:\\"|[^"\r\n])*"))|(\s*=[^,)]*))/mg;
const ARGUMENT_NAMES = /([^\s,]+)/g;

/**
 * Get function params
 *
 * @param {Function} fn
 * @param {number} skip Skip params
 * @return {array}
 */
function getFunctionParams(fn, skip = 0) {
  const functionString = fn.toString().replace(STRIP_COMMENTS, '');

  let params = functionString.slice(
    functionString.indexOf('(') + 1,
    functionString.indexOf(')'),
  ).match(ARGUMENT_NAMES);

  if (params === null) {
    params = [];
  }

  const filteredParams = [];
  let openDestructors = 0;
  let skippedCount = 0;

  for (let i = 0; i < params.length; i++) {
    switch (params[i]) {
      case '{':
        openDestructors++;

        break;
      case '}':
        openDestructors--;

        if (openDestructors === 0 && skippedCount < skip) {
          skippedCount++;
        }

        break;
      default:
        if (openDestructors > 0) {
          break;
        }

        if (skippedCount < skip) {
          skippedCount++;

          break;
        }

        filteredParams.push(params[i]);
    }
  }

  return filteredParams;
}

module.exports = getFunctionParams;
