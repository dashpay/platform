const bs58 = require('bs58');

const { default: loadWasmDpp } = require('../../../dist');

describe.skip('generateDataContractId', () => {
  let ownerId;
  let entropy;
  let DataContractFactory;
  let DataContractValidator;

  before(async () => {
    ({
      DataContractFactory, DataContractValidator,
    } = await loadWasmDpp());
  });

  beforeEach(() => {
    ownerId = bs58.decode('4Sr8EZ9BjQpMpBX5PLPxfvv5crTERSWBnRJXxUmCcrCQ');
    entropy = bs58.decode('85NSrhBXYJAwzj8rDXuYhPQZPyg67diKSNcfhfnVhKnT');
  });

  it('should generate bs58 id based on ', () => {
    const id = bs58.decode('8W4qubpTaFKEWHqr9vAFSNGFULnz2TbgbG4eUik7yWx2');

    const entropyGenerator = {
      generate() {
        return entropy;
      },
    };

    const factory = new DataContractFactory(1337, new DataContractValidator(), entropyGenerator);
    const dataContract = factory.create(ownerId, {});

    expect(Buffer.compare(id, dataContract.getId().toBuffer())).to.equal(0);
  });
});
