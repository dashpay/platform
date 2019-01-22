/* eslint-disable no-unused-expressions */
const rewire = require('rewire');
const chai = require('chai');

const verifyConfig = rewire('../../lib/config/verifyConfig');

const { expect } = chai;

describe('#verifyConfig', () => {
  describe('#verifyHost', () => {
    it('should verifyHost properly handle diff values', () => {
      // eslint-disable-next-line no-underscore-dangle
      const verifyHost = verifyConfig.__get__('verifyHost');
      expect(verifyHost('asd.com')).to.be.true;
      expect(verifyHost('asd')).to.be.false;
      expect(verifyHost('true')).to.be.false;
      expect(verifyHost('127.0.0.1')).to.be.true;
      expect(verifyHost('127.0.0')).to.be.false;
    });
  });
  describe('#verifyURL', () => {
    it('should verifyHost properly handle diff values', () => {
      // eslint-disable-next-line no-underscore-dangle
      const verifyURL = verifyConfig.__get__('verifyURL');
      expect(verifyURL('http;asd.com')).to.be.false;
      expect(verifyURL('asd')).to.be.false;
      expect(verifyURL('true')).to.be.false;
      expect(verifyURL('127.0.0.1')).to.be.true;
      expect(verifyURL('http://127.0.0.1:1231')).to.be.true;
      expect(verifyURL('http://127.0.0.1:fsdfs')).to.be.false;
    });
  });
  describe('#verifyPort', () => {
    it('should verifyPort properly handle diff values', () => {
      // eslint-disable-next-line no-underscore-dangle
      const verifyPort = verifyConfig.__get__('verifyPort');
      expect(verifyPort('1000')).to.be.true;
      expect(verifyPort('asd')).to.be.false;
      expect(verifyPort('true')).to.be.false;
      expect(verifyPort('-1')).to.be.false;
      expect(verifyPort('1')).to.be.true;
      expect(verifyPort('22')).to.be.true;
      expect(verifyPort('654321')).to.be.false;
    });
  });
});
