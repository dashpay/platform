const { BaseTransporter } = require('../../src/transporters');

const methods = [
  'getAddressSummary',
  'getTransaction',
  'getUTXO',
  'subscribeToAddresses',
];
class FakeInvalidTransporter extends BaseTransporter {
  constructor() {
    super({ type: 'FakeInvalidTransporter' });
  }
}
[...methods]
  .forEach((key) => {
    FakeInvalidTransporter.prototype[key] = function () {
      return new Error('DummyFunction');
    };
  });

module.exports = FakeInvalidTransporter;
