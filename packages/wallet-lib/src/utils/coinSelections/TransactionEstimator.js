const _ = require('lodash');
const {
  Address, Script, Transaction,
} = require('@dashevo/dashcore-lib');
const logger = require('../../logger');
const {
  FEES,
  VERSION_BYTES,
  TXOUT_DUFFS_VALUE_BYTES,
  N_LOCKTIME_BYTES,
  TXIN_OUTPOINT_TXID_BYTES,
  TXIN_OUTPOINT_INDEX_BYTES,
  TXIN_SEQUENCE_BYTES,
} = require('../../CONSTANTS');
const is = require('../is');
const { varIntSizeBytesFromLength } = require('../varInt');

const { Output } = Transaction;
const calculateInputsSize = (inputs) => {
  let inputsSize = 0;
  inputs.forEach(() => {
    // eslint-disable-next-line new-cap
    const scriptPubKeyBytes = 100;// On average it's ~80
    const scriptPubKeyLengthBytes = varIntSizeBytesFromLength(scriptPubKeyBytes);

    const inputBytes = TXIN_OUTPOINT_TXID_BYTES
      + TXIN_OUTPOINT_INDEX_BYTES
      + scriptPubKeyLengthBytes
      + scriptPubKeyBytes
      + TXIN_SEQUENCE_BYTES;

    inputsSize += inputBytes;
  });
  return varIntSizeBytesFromLength(inputs.length) + inputsSize;
};
const calculateOutputsSize = (outputs, tx) => {
  let outputsBytes = 0;
  outputs.forEach((output) => {
    const address = (output.address instanceof Address)
      ? output.address
      : Address.fromString(output.address);
    const pkScript = Script.buildPublicKeyHashOut(address).toBuffer();
    const pkScriptSigBytes = pkScript.length;
    const pkScriptLengthBytes = varIntSizeBytesFromLength(pkScriptSigBytes);
    // eslint-disable-next-line new-cap
    tx.addOutput(new Output.fromObject({
      satoshis: output.satoshis,
      script: pkScript,
    }));

    outputsBytes += (TXOUT_DUFFS_VALUE_BYTES
      + pkScriptLengthBytes
      + pkScriptSigBytes);
  });

  return varIntSizeBytesFromLength(outputs.length) + outputsBytes;
};

const defaultOpts = {
  scriptType: 'P2PKH', // We only support that for now;
};
class TransactionEstimator {
  constructor(feeCategory) {
    this.state = {
      outputs: [],
      inputs: [],
    };
    this.feeCategory = feeCategory;
  }

  reduceFeeFromOutput(amoutToReduce) {
    const output = this.state.outputs[0];
    output.satoshis -= amoutToReduce;
  }

  getOutputs() {
    return this.state.outputs;
  }

  getInputs() {
    return this.state.inputs;
  }

  getInValue() {
    return this.state.inputs.reduce((prev, curr) => prev + curr.satoshis, 0);
  }

  getOutValue() {
    return this.state.outputs.reduce((prev, curr) => prev + curr.satoshis, 0);
  }

  addInputs(_inputs = []) {
    const self = this;
    const inputs = (is.arr(_inputs)) ? _inputs : [_inputs];
    if (inputs.length < 1) return false;

    const addInput = (input) => {
      if (!(input instanceof Transaction.UnspentOutput)) {
        throw new Error('Expected valid UnspentOutput to import');
      }
      self.state.inputs.push(input);
    };

    inputs.forEach(addInput);
    return inputs;
  }

  addOutputs(_outputs = []) {
    const self = this;
    const outputs = (is.arr(_outputs)) ? _outputs : [_outputs];
    if (outputs.length < 1) return false;

    const addOutput = (output) => {
      if (!_.has(output, 'scriptType')) {
        // eslint-disable-next-line no-param-reassign
        output.scriptType = defaultOpts.scriptType;
      }
      self.state.outputs.push(output);
    };

    outputs.forEach(addOutput);
    return outputs;
  }

  getSize() {
    const tx = new Transaction();

    let size = 0;
    size += VERSION_BYTES;
    // DIP3
    // size += VERSION_BYTES_DIP3
    // size += TYPE_BYTES_DIP3
    size += calculateInputsSize(this.state.inputs);
    size += calculateOutputsSize(this.state.outputs, tx);
    size += 16;
    // size += calculateExtraPayload(this.state.extraPayload);
    size += N_LOCKTIME_BYTES;

    return size;
  }

  getTotalOutputValue() {
    let totalValue = 0;
    this.state.outputs.forEach((output) => {
      totalValue += output.satoshis;
    });
    return totalValue;
  }

  getFeeEstimate() {
    return this.estimateFees();
  }

  estimateFees() {
    const bytesSize = this.getSize();
    if (this.feeCategory === 'instant') {
      const inputNb = this.getInputs().length;
      return (inputNb * FEES.INSTANT_FEE_PER_INPUTS);
    }
    return ((bytesSize / 1000) * FEES[this.feeCategory.toUpperCase()]);
  }

  debug() {
    logger.info('=== Transaction Estimator');
    logger.info('State:', this.state);
    logger.info('Size', this.getSize());
    logger.info('Fees', this.estimateFees());
    logger.info('=========================');
  }
}
module.exports = TransactionEstimator;
