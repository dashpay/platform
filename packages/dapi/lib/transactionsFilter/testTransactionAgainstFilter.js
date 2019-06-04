const Filter = require('bloom-filter');

/**
 * @param {string} transactionHash
 * @param {Number} inputIndex
 * @returns {Buffer}
 */
function inputIndexToBuffer(transactionHash, inputIndex) {
  const binaryTransactionHash = Buffer.from(transactionHash, 'hex');
  const indexBuffer = Buffer.alloc(4);

  indexBuffer.writeUInt32LE(inputIndex, 0);

  return Buffer.concat([binaryTransactionHash, indexBuffer]);
}

/**
 * @param {Filter} filter
 * @param {Script} script
 * @returns {boolean}
 */
function filterContainsScript(filter, script) {
  const matchedChunk = script.chunks.find((chunk) => {
    if (chunk.opcodenum === 0 || !chunk.buf) {
      return false;
    }

    return filter.contains(chunk.buf);
  });

  return Boolean(matchedChunk);
}

/**
 * @param {Filter} filter
 * @param {Transaction} transaction
 * @returns {boolean}
 */
function checkOutputs(filter, transaction) {
  if (!Array.isArray(transaction.outputs)) {
    return false;
  }

  const matchedOutput = transaction.outputs.find((output, index) => {
    const isMatchFound = filterContainsScript(filter, output.script);

    const alwaysUpdateFilterOnMatch = filter.nFlags === Filter.BLOOM_UPDATE_ALL;
    const updateFilterOnPubKeyMatch = filter.nFlags === Filter.BLOOM_UPDATE_P2PUBKEY_ONLY;

    const isScriptPubKeyOut = output.script.isPublicKeyOut() || output.script.isMultisigOut();

    const isFilterUpdateNeeded = alwaysUpdateFilterOnMatch
      || (updateFilterOnPubKeyMatch && isScriptPubKeyOut);

    if (isMatchFound && isFilterUpdateNeeded) {
      filter.insert(inputIndexToBuffer(transaction.hash, index));
    }

    return isMatchFound;
  });

  return Boolean(matchedOutput);
}

/**
 * @param {Filter} filter
 * @param {Transaction} transaction
 * @return boolean
 */
function checkInputs(filter, transaction) {
  if (!Array.isArray(transaction.inputs)) {
    return false;
  }

  const matchedInput = transaction.inputs.find((input) => {
    const isPrevTxExist = Boolean(input.prevTxId);
    const containsPreviousOutput = isPrevTxExist && filter.contains(
      inputIndexToBuffer(input.prevTxId, input.outputIndex),
    );

    return containsPreviousOutput || filterContainsScript(filter, input.script);
  });

  return Boolean(matchedInput);
}

/**
 * @param {Filter} filter
 * @param {Transaction} transaction
 * @return boolean
 */
function checkHash(filter, transaction) {
  const binaryHash = Buffer.from(transaction.hash, 'hex');

  return filter.contains(binaryHash);
}

/**
 * BIP37 transaction filtering
 *
 * @type testFunction
 * @param {Filter} filter
 * @param {Transaction} data
 * @return boolean
 */
function testTransactionAgainstFilter(filter, data) {
  return checkHash(filter, data)
    || checkOutputs(filter, data)
    || checkInputs(filter, data);
}

module.exports = testTransactionAgainstFilter;
