/**
 * Generate JSON with big depth
 * @param {number} depth
 * @returns {object}
 */
function generateDeepJson(depth) {
  const result = {};

  if (depth === 1) {
    return {
      depth,
    };
  }

  result[depth] = generateDeepJson(depth - 1);

  return result;
}

module.exports = generateDeepJson;
