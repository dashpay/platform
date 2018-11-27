/* eslint-disable global-require */
process.on('unhandledRejection', (up) => { throw up; });
const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const sinon = require('sinon');
const proxyquire = require('proxyquire');
const coreAPIFixture = require('../../fixtures/coreAPIFixture');


const { expect } = chai;
chai.use(chaiAsPromised);
let spy;
let stub;

describe('api/dashcore/rpc', async () => {
  const config = require('../../../lib/config');
  config.dashcore.rpc.port = 12345;
  proxyquire('../../../lib/api/dashcore/rpc', { '../../config': config });
  const rpc = require('../../../lib/api/dashcore/rpc');

  describe('getHashFromHeight', () => {
    describe('#factory', () => {
      it('should return a promise', async () => {
        const getHashFromHeight = rpc.getHashFromHeight();
        expect(getHashFromHeight).to.be.a('promise');
      });
      it('should return error with invalid height', async () => {
        const getHashFromHeight = rpc.getHashFromHeight('1');
        await expect(getHashFromHeight).to.be.rejectedWith('JSON');
        expect(spy.callCount).to.be.equal(1);
      });
      it('should return error with invalid height', async () => {
        const getHashFromHeight = rpc.getHashFromHeight('str');
        await expect(getHashFromHeight).to.be.rejectedWith('JSON');
        expect(spy.callCount).to.be.equal(1);
      });
      it('Should return a hash', async () => {
        expect(spy.callCount).to.be.equal(0);
        await expect(rpc.getHashFromHeight(1)).to.be.rejectedWith('JSON');
        expect(spy.callCount).to.be.equal(1);
      });
    });
    before(() => {
      spy = sinon.spy(rpc, 'getHashFromHeight');
    });
    beforeEach(() => {
      spy.resetHistory();
    });
    after(() => {
      spy.restore();
    });
  });

  describe('getTransaction', () => {
    const txHash = '50622f66236671501c0e80f388d6cf1e81158de8526f4acc9db00adf3c524077';
    const tx = 'ygPcCwVy7Fxg7ruxZzqVYdPLtvw7auHAFh';
    describe('#factory', () => {
      it('should return a promise', () => {
        const getTransaction = rpc.getTransaction();
        expect(getTransaction).to.be.a('promise');
      });
      it('should return error with invalid transaction', async () => {
        const getHashFromHeight = rpc.getTransaction(1);
        await expect(getHashFromHeight).to.be.rejectedWith('JSON');
      });
    });
    describe('#stub', () => {
      before(() => {
        stub = sinon.stub(rpc, 'getTransaction');
        stub.rejects(new Error('Invalid or non-wallet transaction id'));
        stub
          .withArgs(txHash)
          .returns(new Promise(resolve => resolve(tx)));
      });

      beforeEach(() => {
        stub.resetHistory();
      });

      after(() => {
        stub.restore();
      });

      it('Should return a hash', async () => {
        rpc.getTransaction(123);
        expect(stub.callCount).to.be.equal(1);
        const transaction = await rpc.getTransaction(txHash);
        expect(transaction).to.be.an('string');
        expect(transaction).to.be.equal(tx);
        expect(stub.callCount).to.be.equal(2);
      });

      it('Should throw if arguments are not valid', async () => {
        const transaction = rpc.getTransaction('str');
        await expect(transaction).to.be.rejectedWith('Invalid or non-wallet transaction id');
        expect(stub.callCount).to.be.equal(1);
        await expect(rpc.getTransaction([])).to.be.rejectedWith('Invalid or non-wallet transaction id');
        expect(stub.callCount).to.be.equal(2);
        await expect(rpc.getTransaction({})).to.be.rejectedWith('Invalid or non-wallet transaction id');
        expect(stub.callCount).to.be.equal(3);
        await expect(rpc.getTransaction({ address: 1 })).to.be.rejectedWith('Invalid or non-wallet transaction id');
        expect(stub.callCount).to.be.equal(4);
        await expect(rpc.getTransaction(coreAPIFixture)).to.be.rejectedWith('Invalid or non-wallet transaction id');
        expect(stub.callCount).to.be.equal(5);
        await expect(rpc.getTransaction(true)).to.be.rejectedWith('Invalid or non-wallet transaction id');
        expect(stub.callCount).to.be.equal(6);
      });
    });
  });

  describe('getTransactionFirstInputAddress', () => {
    describe('#factory', () => {
      it('should return a promise', () => {
        const res = rpc.getTransactionFirstInputAddress();
        expect(res).to.be.a('promise');
      });
      it('should return error with invalid transaction', async () => {
        const res = rpc.getTransactionFirstInputAddress(1);
        await expect(res).to.be.rejectedWith('JSON');
      });
    });

    describe('#stub', () => {
      const txHash = '50622f66236671501c0e80f388d6cf1e81158de8526f4acc9db00adf3c524077';
      const addrStr = 'ygPcCwVy7Fxg7ruxZzqVYdPLtvw7auHAFh';
      before(() => {
        stub = sinon.stub(rpc, 'getTransactionFirstInputAddress');
        stub
          .withArgs(addrStr)
          .returns(new Promise(resolve => resolve(txHash)));
      });

      beforeEach(() => {
        stub.resetHistory();
      });

      after(() => {
        stub.restore();
      });

      it('Should return a hash', async () => {
        rpc.getTransactionFirstInputAddress(123);
        expect(stub.callCount).to.be.equal(1);
        const transaction = await rpc.getTransactionFirstInputAddress(addrStr);
        expect(stub.callCount).to.be.equal(2);
      });
    });
  });

  describe('getCurrentBlockHeight', () => {
    describe('#factory', () => {
      it('should return a promise', () => {
        const res = rpc.getCurrentBlockHeight();
        expect(res).to.be.a('promise');
      });
      it('should return error with invalid transaction', async () => {
        const res = rpc.getCurrentBlockHeight();
        await expect(res).to.be.rejectedWith('JSON');
      });
    });

    describe('#stub', () => {
      before(() => {
        stub = sinon.stub(rpc, 'getCurrentBlockHeight');
        stub.returns(new Promise(resolve => resolve('fake')));
      });

      beforeEach(() => {
        stub.resetHistory();
      });

      after(() => {
        stub.restore();
      });

      it('Should return a hash', async () => {
        rpc.getCurrentBlockHeight();
        expect(stub.callCount).to.be.equal(1);
        const transaction = await rpc.getCurrentBlockHeight();
        expect(stub.callCount).to.be.equal(2);
      });
    });
  });

  describe('getMasternodesList', () => {
    describe('#factory', () => {
      it('should return a promise', () => {
        const res = rpc.getMasternodesList();
        expect(res).to.be.a('promise');
      });
      it('should return error with invalid transaction', async () => {
        const res = rpc.getMasternodesList();
        await expect(res).to.be.rejectedWith('JSON');
      });
    });

    describe('#stub', () => {
      before(() => {
        stub = sinon.stub(rpc, 'getMasternodesList');
        stub.returns(new Promise(resolve => resolve('fake')));
      });

      beforeEach(() => {
        stub.resetHistory();
      });

      after(() => {
        stub.restore();
      });

      it('Should return a hash', async () => {
        rpc.getMasternodesList();
        expect(stub.callCount).to.be.equal(1);
        const transaction = await rpc.getMasternodesList();
        expect(stub.callCount).to.be.equal(2);
      });
    });
  });

  describe('getUTXO', () => {
    describe('#factory', () => {
      it('should return a promise', () => {
        const res = rpc.getUTXO();
        expect(res).to.be.a('promise');
      });
      it('should return error with invalid transaction', async () => {
        const res = rpc.getUTXO(1);
        await expect(res).to.be.rejectedWith('JSON');
      });
    });

    describe('#stub', () => {
      const addrStr = 'ygPcCwVy7Fxg7ruxZzqVYdPLtvw7auHAFh';
      before(() => {
        stub = sinon.stub(rpc, 'getUTXO');
        stub
          .withArgs(addrStr)
          .returns(new Promise(resolve => resolve('fake')));
      });

      beforeEach(() => {
        stub.resetHistory();
      });

      after(() => {
        stub.restore();
      });

      it('Should return a hash', async () => {
        rpc.getUTXO(123);
        expect(stub.callCount).to.be.equal(1);
        const transaction = await rpc.getUTXO(addrStr);
        expect(stub.callCount).to.be.equal(2);
      });
    });
  });

  describe('getBlockHash', () => {
    describe('#factory', () => {
      it('should return a promise', () => {
        const res = rpc.getBlockHash();
        expect(res).to.be.a('promise');
      });
      it('should return error with invalid transaction', async () => {
        const res = rpc.getBlockHash(1);
        await expect(res).to.be.rejectedWith('JSON');
      });
    });

    describe('#stub', () => {
      before(() => {
        stub = sinon.stub(rpc, 'getBlockHash');
        stub
          .withArgs(1)
          .returns(new Promise(resolve => resolve('fake')));
      });

      beforeEach(() => {
        stub.resetHistory();
      });

      after(() => {
        stub.restore();
      });

      it('Should return a hash', async () => {
        rpc.getBlockHash(123);
        expect(stub.callCount).to.be.equal(1);
        const transaction = await rpc.getBlockHash(324);
        expect(stub.callCount).to.be.equal(2);
      });
    });
  });

  describe('getBlockHash', () => {
    describe('#factory', () => {
      it('should return a promise', () => {
        const res = rpc.getBlockHash();
        expect(res).to.be.a('promise');
      });
      it('should return error with invalid transaction', async () => {
        const res = rpc.getBlockHash(1);
        await expect(res).to.be.rejectedWith('JSON');
      });
    });
  });

  describe('getBlock', () => {
    describe('#factory', () => {
      it('should return a promise', () => {
        const res = rpc.getBlock();
        expect(res).to.be.a('promise');
      });
      it('should return error with invalid transaction', async () => {
        const res = rpc.getBlock(1);
        await expect(res).to.be.rejectedWith('JSON');
      });
    });

    describe('#stub', () => {
      const txHash = '50622f66236671501c0e80f388d6cf1e81158de8526f4acc9db00adf3c524077';
      before(() => {
        stub = sinon.stub(rpc, 'getBlock');
        const isParsedValue = 1;
        stub
          .withArgs(txHash, isParsedValue)
          .returns(new Promise(resolve => resolve('fake')));
      });

      beforeEach(() => {
        stub.resetHistory();
      });

      after(() => {
        stub.restore();
      });

      it('Should callCount be correct', async () => {
        rpc.getBlock(123);
        expect(stub.callCount).to.be.equal(1);
        const transaction = await rpc.getBlock(324);
        expect(stub.callCount).to.be.equal(2);
      });
    });
  });

  describe('getrawtransaction', () => {
    describe('#factory', () => {
      it('should return a promise', () => {
        const res = rpc.getRawTransaction();
        expect(res).to.be.a('promise');
      });
      it('should return error with invalid transaction', async () => {
        const res = rpc.getRawTransaction(1);
        await expect(res).to.be.rejectedWith('JSON');
      });
    });

    describe('#stub', () => {
      const tsid = '50622f66236671501c0e80f388d6cf1e81158de8526f4acc9db00adf3c524077';
      before(() => {
        stub = sinon.stub(rpc, 'getRawTransaction');
        stub
          .withArgs(tsid)
          .returns(new Promise(resolve => resolve('fake')));
      });

      beforeEach(() => {
        stub.resetHistory();
      });

      after(() => {
        stub.restore();
      });

      it('Should callCount be correct', async () => {
        rpc.getRawTransaction('123');
        expect(stub.callCount).to.be.equal(1);
        const transaction = await rpc.getRawTransaction(tsid);
        expect(stub.callCount).to.be.equal(2);
      });
    });
  });

  describe('getRawBlock', () => {
    describe('#factory', () => {
      it('should return a promise', () => {
        const res = rpc.getRawBlock();
        expect(res).to.be.a('promise');
      });
      it('should return error with invalid transaction', async () => {
        const res = rpc.getRawBlock(1);
        await expect(res).to.be.rejectedWith('JSON');
      });
    });

    describe('#stub', () => {
      const tsid = '50622f66236671501c0e80f388d6cf1e81158de8526f4acc9db00adf3c524077';
      before(() => {
        stub = sinon.stub(rpc, 'getRawBlock');

        stub
          .withArgs(tsid)
          .returns(new Promise(resolve => resolve('fake')));
      });

      beforeEach(() => {
        stub.resetHistory();
      });

      after(() => {
        stub.restore();
      });

      it('Should callCount be correct', async () => {
        rpc.getRawBlock('123');
        expect(stub.callCount).to.be.equal(1);
        const transaction = await rpc.getRawBlock(tsid);
        expect(stub.callCount).to.be.equal(2);
      });
    });
  });

  describe('sendRawTransaction', () => {
    describe('#factory', () => {
      it('should return a promise', () => {
        const res = rpc.sendRawTransaction();
        expect(res).to.be.a('promise');
      });
      it('should return error with invalid transaction', async () => {
        const res = rpc.sendRawTransaction(1);
        await expect(res).to.be.rejectedWith('JSON');
      });
    });

    describe('#stub', () => {
      const tsid = '50622f66236671501c0e80f388d6cf1e81158de8526f4acc9db00adf3c524077';
      before(() => {
        stub = sinon.stub(rpc, 'sendRawTransaction');
        stub
          .withArgs(tsid)
          .returns(new Promise(resolve => resolve('fake')));
      });

      beforeEach(() => {
        stub.resetHistory();
      });

      after(() => {
        stub.restore();
      });

      it('Should callCount be correct', async () => {
        rpc.sendRawTransaction('123');
        expect(stub.callCount).to.be.equal(1);
        const transaction = await rpc.sendRawTransaction(tsid);
        expect(stub.callCount).to.be.equal(2);
      });
    });
  });
});
