/* eslint-disable no-unused-expressions */
// Suppressed to use chai without dirty-chai
// TODO: Move to Jest instead of mocha/chai/sinon/nyc/etc
const chai = require('chai');

const utils = require('../../lib/utils');

const { expect } = chai;

describe('utils', () => {
  describe('#isRegtest', () => {
    it('Should return true only if "regtest" string passed', () => {
      expect(utils.isRegtest('regtest')).to.be.true;

      expect(utils.isRegtest('regtest=')).to.be.false;
      expect(utils.isRegtest('_regtest')).to.be.false;
      expect(utils.isRegtest('retest')).to.be.false;
      expect(utils.isRegtest('devnet')).to.be.false;
      expect(utils.isRegtest('mainnet')).to.be.false;
      expect(utils.isRegtest('testnet')).to.be.false;
    });
  });
  describe('#isDevnet', () => {
    it('Should return true only if string that starts from "devnet" passed', () => {
      expect(utils.isDevnet('devnet')).to.be.true;
      expect(utils.isDevnet('devnet=mysuperdevnet')).to.be.true;

      expect(utils.isDevnet('_devnet')).to.be.false;
      expect(utils.isDevnet('regtest=')).to.be.false;
      expect(utils.isDevnet('_regtest')).to.be.false;
      expect(utils.isDevnet('retest')).to.be.false;
      expect(utils.isDevnet('mainnet')).to.be.false;
      expect(utils.isDevnet('testnet')).to.be.false;
    });
  });
});
