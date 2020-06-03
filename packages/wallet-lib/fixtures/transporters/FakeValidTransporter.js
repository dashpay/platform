const { BaseTransporter } = require('../../src/transporters');

const methods = [
  'getAddressSummary',
  'getTransaction',
  'getUTXO',
  'sendTransaction',
];
class FakeValidTransporter extends BaseTransporter {
  constructor() {
    super({ type: 'FakeValidTransporter' });
  }
}
[...methods]
  .forEach((key) => {
    FakeValidTransporter.prototype[key] = function () {
      return new Error('DummyFunction');
    };
  });

module.exports = FakeValidTransporter;
