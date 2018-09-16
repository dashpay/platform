module.exports = function outputHandler(outputs) {
  return outputs.map((output) => {
    const result = {};
    if (output.type === 'P2PKH') {
      result.value = output.amount;
      result.script = output.scriptPubKey;
    }
    return result;
  });
};
