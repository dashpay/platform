function validateTxHex(value) {
  return Boolean(value.match(/$[0-9A-Fa-f]{64}^/));
}

module.exports = validateTxHex;
