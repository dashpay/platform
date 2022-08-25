/* eslint-disable no-unused-expressions */
const chai = require('chai');

const { getConfigFixture } = require('../../mocks/config');

const { validateConfig, validateHost, validatePort } = require('../../../lib/config/validator');

const { expect } = chai;

describe('config/validator', () => {
  describe('validateConfig', () => {
    it('Should return an object with isValid and validationErrors fields', () => {
      const config = getConfigFixture();

      const validationResult = validateConfig(config);

      expect(validationResult).to.have.a.property('isValid');
      expect(validationResult.isValid).to.be.a('boolean');

      expect(validationResult).to.have.a.property('validationErrors');
      expect(validationResult.validationErrors).to.be.an('array');
    });
    it('Should return and empty array in validationErrors if there is no errors', () => {
      const config = getConfigFixture();

      const validationResult = validateConfig(config);

      expect(validationResult.isValid).to.be.true;
      expect(validationResult.validationErrors.length).to.be.equal(0);
    });
    it('Should return errors in array if there are invalid fields in the config', () => {
      const config = getConfigFixture();

      config.dashcore.p2p.host = 1;
      config.dashcore.p2p.port = '$/*';
      const validationResult = validateConfig(config);

      expect(validationResult.isValid).to.be.false;
      expect(validationResult.validationErrors.length).to.be.equal(2);
    });
  });
  describe('validateHost', () => {
    it('Should return true in isValid field that host is an alphanumeric value', () => {
      // It is an alphanumeric to support docker
      // eslint-disable-next-line no-underscore-dangle
      expect(validateHost('asd.com').isValid).to.be.true;
      expect(validateHost('127.0.0.1').isValid).to.be.true;
      expect(validateHost('asd').isValid).to.be.true;
      expect(validateHost('true').isValid).to.be.true;
      expect(validateHost('127.0.0').isValid).to.be.true;
      expect(validateHost('127.0.0.1:123').isValid).to.be.true;

      expect(validateHost(1).isValid).to.be.false;
      expect(validateHost({}).isValid).to.be.false;
      expect(validateHost(true).isValid).to.be.false;
      expect(validateHost(undefined).isValid).to.be.false;
      expect(validateHost(null).isValid).to.be.false;
      expect(validateHost('').isValid).to.be.false;
    });
  });
  describe('validatePort', () => {
    it('Should return true in isValid field if value is a valid port', () => {
      // eslint-disable-next-line no-underscore-dangle
      expect(validatePort('1000').isValid).to.be.true;
      expect(validatePort('1').isValid).to.be.true;
      expect(validatePort('22').isValid).to.be.true;

      expect(validatePort('asd').isValid).to.be.false;
      expect(validatePort('true').isValid).to.be.false;
      expect(validatePort('-1').isValid).to.be.false;
      expect(validatePort('654321').isValid).to.be.false;
    });
  });
});
