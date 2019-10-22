const path = require('path');

const loadPackageDefinition = require('../../lib/loadPackageDefinition');

describe('loadPackageDefinition', () => {
  let protoPath;

  beforeEach(() => {
    protoPath = path.join(__dirname, '../../lib/test/fixture/example.proto');
  });

  it('should successfuly load package definition', () => {
    const definition = loadPackageDefinition(protoPath, 'org.dash.platform.example.v0');

    expect(definition.Example).to.be.an.instanceOf(Function);
    expect(definition.Example).to.have.a.property('service');
  });
});
