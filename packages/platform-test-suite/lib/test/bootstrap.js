const path = require('path');
const dotenvSafe = require('dotenv-safe');
const { expect, use } = require('chai');
const dirtyChai = require('dirty-chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const sinonChai = require('sinon-chai');

const wasmDpp = require('@dashevo/wasm-dpp');

use(chaiAsPromised);
use(dirtyChai);
use(sinonChai);

dotenvSafe.config({
  path: path.resolve(__dirname, '..', '..', '.env'),
});

process.env.NODE_ENV = 'test';

if (!wasmDpp.deserializeConsensusError.__withSillyDebug) {
  const originalDeserializeConsensusError = wasmDpp.deserializeConsensusError;

  wasmDpp.deserializeConsensusError = function debugDeserializeConsensusError(bytes, ...args) {
    const buffer = bytes ? Buffer.from(bytes) : Buffer.alloc(0);

    console.debug('[consensus-error-debug] will deserialize consensus error bytes', {
      hex: buffer.toString('hex'),
      base64: buffer.toString('base64'),
      length: buffer.length,
      isEmpty: buffer.length === 0,
    });

    try {
      const result = originalDeserializeConsensusError.call(this, bytes, ...args);

      const code = typeof result?.getCode === 'function' ? result.getCode() : undefined;

      console.debug('[consensus-error-debug] deserialized consensus error result', {
        name: result?.constructor?.name,
        code,
        message: result?.message,
      });

      return result;
    } catch (e) {
      console.error('[consensus-error-debug] failed to deserialize consensus error', {
        errorMessage: e?.message,
        stack: e?.stack,
      });

      throw e;
    }
  };

  wasmDpp.deserializeConsensusError.__withSillyDebug = true;
}

let faucetIndex = 1;
if (process.env.MOCHA_WORKER_ID) {
  const mochaWorkerId = parseInt(process.env.MOCHA_WORKER_ID, 10);
  faucetIndex = mochaWorkerId + 1;
}

process.env.FAUCET_ADDRESS = process.env[`FAUCET_${faucetIndex}_ADDRESS`];
process.env.FAUCET_PRIVATE_KEY = process.env[`FAUCET_${faucetIndex}_PRIVATE_KEY`];

exports.mochaHooks = {
  beforeEach() {
    if (!this.sinon) {
      this.sinon = sinon.createSandbox();
    } else {
      this.sinon.restore();
    }
  },

  afterEach() {
    this.sinon.restore();
  },
};

global.expect = expect;
