const sinon = require('sinon');
const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const RPCClient = require('../../src/RPCClient');
const RPCError = require("../../src/errors/RPCError");
const axios = require('axios');

chai.use(chaiAsPromised);
const { expect } = chai;

const testHost = 'stubbed_address';
const testPort = 4567;
const testPath = `http://${testHost}:${testPort}`;

describe('RPCClient', async () => {

  before(() => {
    const axiosStub = sinon.stub(axios, 'post');
    axiosStub.withArgs(testPath, {jsonrpc: '2.0', method: 'test', params:['correct data'], id: 1}).returns(new Promise((resolve) => {
      resolve({ status: 200, data: {result: 'passed', error: null} });
    }));
    axiosStub.withArgs(testPath, {jsonrpc: '2.0', method: 'test', params:['wrong data'], id: 1}).returns(new Promise((resolve) => {
      resolve({ status: 400, data: {result: null, error: { message: 'wrong data'}} });
    }));
    axiosStub.withArgs(testPath, {jsonrpc: '2.0', method: 'test', params:['invalid data'], id: 1}).returns(new Promise((resolve) => {
      resolve({ status: 200, data: {result: 'passed', error: { message: 'Invalid data' }} });
    }));
    axiosStub.withArgs(testPath, {jsonrpc: '2.0', method: 'test', params:['invalid data for error.data'], id: 1}).returns(new Promise((resolve) => {
      resolve({ status: 200, data: {result: 'passed', error: { message: 'Invalid data for error.data', data: "additional data here" }} });
    }));
  });

  after(() => {
    axios.post.restore();
  });

  describe('.request()', async() => {
    it('Should make rpc requests and return result if first arg is options', async() => {
      const result = await RPCClient.request({
        host: 'stubbed_address',
        port: 4567
      }, 'test', ['correct data']);
      expect(result).to.equal('passed');
    });
    it('Should make rpc requests and return result if first arg is url', async() => {
      const result = await RPCClient.request(testPath, 'test', ['correct data']);
      expect(result).to.equal('passed');
    });
    it('Should throw if response status is not 200', async() => {
      const promise = RPCClient.request({
        host: 'stubbed_address',
        port: 4567
      }, 'test', ['wrong data']);
      await expect(promise).to.be.rejectedWith(Error, '');
    });
    it('Should throw if there is error object in response body', async() => {
      const promise = RPCClient.request({
        host: 'stubbed_address',
        port: 4567
      }, 'test', ['invalid data']);
      try{
        await promise;
      } catch (err) {
        expect(err).to.be.an.instanceof(RPCError);
        expect(err.message).to.equal('DAPI RPC error: test: Invalid data');
        expect(err.getData()).to.equal(undefined);
      }
    });
    it('Should throw if there is error object with data in response body', async() => {
      const promise = RPCClient.request({
        host: 'stubbed_address',
        port: 4567
      }, 'test', ['invalid data for error.data']);
      try{
        await promise;
      } catch (err) {
        expect(err).to.be.an.instanceof(RPCError);
        expect(err.message).to.equal('DAPI RPC error: test: Invalid data for error.data');
        expect(err.getData()).to.equal('additional data here');
      }
    });

  })
});
