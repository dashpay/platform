import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import {
  json, object, id, ownerId,
} from './mocks/DataContract/index.js';
import { fromHexString } from './utils/hex.ts';

let PlatformVersion: typeof wasm.PlatformVersion;

before(async () => {
  await initWasm();
  ({ PlatformVersion } = wasm);
});

let dataContractsBytes: string[];

describe('DataContract', () => {
  before(async () => {
    dataContractsBytes = [
      '003662bb61e17fae3ea294cf603197fb0aab6d51180bd8b6104c4944a62fe2d97f00000000000101000001000000000000000000000000000000000000000000000000000000000000000000010a7769746864726177616c1607120b6465736372697074696f6e12805769746864726177616c20646f63756d656e7420746f20747261636b20756e6465726c79696e67207769746864726177616c207472616e73616374696f6e732e205769746864726177616c732073686f756c6420626520637265617465642077697468204964656e746974795769746864726177616c5472616e736974696f6e12176372656174696f6e5265737472696374696f6e4d6f6465020212047479706512066f626a6563741207696e64696365731504160312046e616d65120e6964656e74697479537461747573120a70726f70657274696573150316011208246f776e6572496412036173631601120673746174757312036173631601120a2463726561746564417412036173631206756e697175651300160312046e616d65120e6964656e74697479526563656e74120a70726f70657274696573150316011208246f776e6572496412036173631601120a2475706461746564417412036173631601120673746174757312036173631206756e697175651300160312046e616d651207706f6f6c696e67120a70726f70657274696573150416011206737461747573120361736316011207706f6f6c696e6712036173631601120e636f72654665655065724279746512036173631601120a2475706461746564417412036173631206756e697175651300160312046e616d65120b7472616e73616374696f6e120a70726f706572746965731502160112067374617475731203617363160112107472616e73616374696f6e496e64657812036173631206756e697175651300120a70726f70657274696573160712107472616e73616374696f6e496e64657816041204747970651207696e7465676572120b6465736372697074696f6e127953657175656e7469616c20696e646578206f6620617373657420756e6c6f636b20287769746864726177616c29207472616e73616374696f6e2e20506f70756c61746564207768656e2061207769746864726177616c20706f6f6c656420696e746f207769746864726177616c207472616e73616374696f6e12076d696e696d756d02011208706f736974696f6e020012157472616e73616374696f6e5369676e48656967687416041204747970651207696e7465676572120b6465736372697074696f6e122f54686520436f726520686569676874206f6e207768696368207472616e73616374696f6e20776173207369676e656412076d696e696d756d02011208706f736974696f6e02011206616d6f756e7416041204747970651207696e7465676572120b6465736372697074696f6e121a54686520616d6f756e7420746f2062652077697468647261776e12076d696e696d756d02fb03e81208706f736974696f6e0202120e636f72654665655065724279746516051204747970651207696e7465676572120b6465736372697074696f6e1250546869732069732074686520666565207468617420796f75206172652077696c6c696e6720746f207370656e6420666f722074686973207472616e73616374696f6e20696e2044756666732f4279746512076d696e696d756d020112076d6178696d756d02fcffffffff1208706f736974696f6e02031207706f6f6c696e6716041204747970651207696e7465676572120b6465736372697074696f6e124e5468697320696e6469636174656420746865206c6576656c20617420776869636820506c6174666f726d2073686f756c642074727920746f20706f6f6c2074686973207472616e73616374696f6e1204656e756d15030200020102021208706f736974696f6e0204120c6f75747075745363726970741605120474797065120561727261791209627974654172726179130112086d696e4974656d73021712086d61784974656d7302191208706f736974696f6e0205120673746174757316041204747970651207696e74656765721204656e756d150502000201020202030204120b6465736372697074696f6e124330202d2050656e64696e672c2031202d205369676e65642c2032202d2042726f61646361737465642c2033202d20436f6d706c6574652c2034202d20457870697265641208706f736974696f6e020612146164646974696f6e616c50726f706572746965731300120872657175697265641507120a24637265617465644174120a247570646174656441741206616d6f756e74120e636f7265466565506572427974651207706f6f6c696e67120c6f75747075745363726970741206737461747573',
    ];
  });

  describe('constructor()', () => {
    it('should allow to create DataContract from schema without full validation', () => {
      const dataContract = new wasm.DataContract({
        ownerId: object.ownerId,
        identityNonce: BigInt(2),
        schemas: object.documentSchemas,
        definitions: null,
        fullValidation: false,
      });

      expect(dataContract).to.be.an.instanceof(wasm.DataContract);
    });

    it('should allow to create DataContract from schema with full validation', () => {
      const dataContract = new wasm.DataContract({
        ownerId: object.ownerId,
        identityNonce: BigInt(2),
        schemas: object.documentSchemas,
        definitions: null,
        fullValidation: true,
      });

      expect(dataContract).to.be.an.instanceof(wasm.DataContract);
    });
  });

  describe('fromJSON()', () => {
    it('should allow to create DataContract from value with full validation and without platform version', () => {
      const dataContract = wasm.DataContract.fromJSON(json, true);

      expect(dataContract).to.be.an.instanceof(wasm.DataContract);
    });
  });

  describe('fromBytes()', () => {
    it('should allow to convert DataContract to bytes and from bytes', () => {
      const [dataContractBytes] = dataContractsBytes;

      const dataContract = wasm.DataContract.fromJSON(json, true);

      expect(dataContract.toBytes(new PlatformVersion(1))).to.deep.equal(fromHexString(dataContractBytes));

      const dataContractFromBytes = wasm.DataContract.fromBytes(
        dataContract.toBytes(new PlatformVersion(1)),
        false,
        new PlatformVersion(1),
      );

      expect(dataContract).to.be.an.instanceof(wasm.DataContract);

      expect(dataContractFromBytes.toBytes(new PlatformVersion(1))).to.deep.equal(fromHexString(dataContractBytes));
    });

    it('should allow to create DataContract from bytes without full validation', () => {
      const [dataContractBytes] = dataContractsBytes;

      const dataContractFromBytes = wasm.DataContract.fromBytes(
        fromHexString(dataContractBytes),
        false,
        new PlatformVersion(1),
      );
      const dataContractFromValue = wasm.DataContract.fromJSON(json, true);

      expect(dataContractFromBytes.toObject()).to.deep.equal(dataContractFromValue.toObject());
    });

    it('should allow to create DataContract from bytes with full validation and without version', () => {
      const [dataContractBytes] = dataContractsBytes;

      const dataContractFromBytes = wasm.DataContract.fromBytes(
        fromHexString(dataContractBytes),
        true,
      );
      const dataContractFromValue = wasm.DataContract.fromJSON(json, true);

      expect(dataContractFromBytes.toObject()).to.deep.equal(dataContractFromValue.toObject());
    });
  });

  describe('fromObject()', () => {
    it('should allow to create DataContract from object produced by toObject', () => {
      const originalContract = wasm.DataContract.fromJSON(json, true);
      const objectRepresentation = originalContract.toObject(new PlatformVersion(1));

      const reconstructedContract = wasm.DataContract.fromObject(
        objectRepresentation,
        true,
        new PlatformVersion(1),
      );

      expect(reconstructedContract.toObject(new PlatformVersion(1))).to.deep.equal(
        objectRepresentation,
      );
    });
  });

  describe('toBase64()', () => {
    it('should allow to convert DataContract to base64 and from base64', () => {
      const [dataContractBytes] = dataContractsBytes;

      const dataContract = wasm.DataContract.fromBytes(
        fromHexString(dataContractBytes),
        true,
        new PlatformVersion(1),
      );

      const base64 = dataContract.toBase64(new PlatformVersion(1));
      const bytes = dataContract.toBytes(new PlatformVersion(1));
      const bytesFromBase64 = Buffer.from(base64, 'base64');

      expect(bytesFromBase64).to.deep.equal(Buffer.from(bytes));

      const dataContractFromBase64 = wasm.DataContract.fromBase64(
        base64,
        true,
        new PlatformVersion(1),
      );

      const actualBytes = Buffer.from(
        dataContractFromBase64.toBytes(new PlatformVersion(1)),
      );
      expect(actualBytes).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('toJSON()', () => {
    it('should allow to get json', () => {
      const dataContract = wasm.DataContract.fromJSON(json, true);

      expect(dataContract.toJSON(new PlatformVersion(1))).to.deep.equal(json);
    });
  });

  describe('schemas', () => {
    it('should allow to get schemas', () => {
      const dataContract = wasm.DataContract.fromJSON(json, true);

      const { schemas } = dataContract;

      expect(schemas).to.deep.equal(object.documentSchemas);
    });
  });

  describe('version', () => {
    it('should allow to get version', () => {
      const dataContract = wasm.DataContract.fromJSON(json, true);

      expect(dataContract.version).to.deep.equal(json.version);
    });

    it('should allow to set version', () => {
      const dataContract = wasm.DataContract.fromJSON(json, true);

      dataContract.version = 20;

      expect(dataContract.version).to.equal(20);
    });
  });

  describe('id', () => {
    it('should allow to get id', () => {
      const dataContract = wasm.DataContract.fromJSON(json, true);

      expect(dataContract.id.toBase58()).to.deep.equal(id);
    });

    it('should allow to set id', () => {
      const dataContract = wasm.DataContract.fromJSON(json, true);

      dataContract.id = new wasm.Identifier('7ckT6Y19HnjfqoPFmfL995i4z2HwgZ8UttNmP99LtCBH');

      expect(dataContract.id.toBase58()).to.deep.equal('7ckT6Y19HnjfqoPFmfL995i4z2HwgZ8UttNmP99LtCBH');
    });
  });

  describe('ownerId', () => {
    it('should allow to get owner id', () => {
      const dataContract = wasm.DataContract.fromJSON(json, true);

      expect(dataContract.ownerId.toBase58()).to.deep.equal(ownerId);
    });

    it('should allow to set owner id', () => {
      const dataContract = wasm.DataContract.fromJSON(json, true);

      dataContract.ownerId = new wasm.Identifier('3bx13Wd5k4LwHAvXJrayc5HdKPyiccKWYECPQGGYfnVL');

      expect(dataContract.ownerId.toBase58()).to.deep.equal(
        '3bx13Wd5k4LwHAvXJrayc5HdKPyiccKWYECPQGGYfnVL',
      );
    });
  });

  describe('config', () => {
    it('should allow to get config', () => {
      const dataContract = wasm.DataContract.fromJSON(json, true);

      const { config } = dataContract;

      expect(config).to.deep.equal(object.config);
    });
  });

  describe('setConfig()', () => {
    it('should allow to set config', () => {
      const dataContract = wasm.DataContract.fromJSON(json, true);

      const oldConfig = dataContract.config;

      const newConfig = { ...oldConfig, canBeDeleted: !oldConfig.canBeDeleted };

      dataContract.setConfig(newConfig, new PlatformVersion(1));

      expect(dataContract.config).to.deep.equal(newConfig);
    });
  });

  describe('setSchemas()', () => {
    it('should allow to set schema', () => {
      const dataContract = wasm.DataContract.fromJSON(json, true);

      // Use JSON fixture for setting (numbers, not BigInt)
      const newSchema = {
        pupup: json.documentSchemas.withdrawal,
      };

      dataContract.setSchemas(newSchema, null, true, new PlatformVersion(1));

      const { schemas } = dataContract;

      // schemas returns object format (BigInt), so compare with object fixture
      const expectedSchema = {
        pupup: object.documentSchemas.withdrawal,
      };

      expect(schemas).to.deep.equal(expectedSchema);
    });
  });

  describe('generateId()', () => {
    it('should allow to generate id', () => {
      const identifier = new wasm.Identifier('3bx13Wd5k4LwHAvXJrayc5HdKPyiccKWYECPQGGYfnVL');

      const generatedId = wasm.DataContract.generateId(identifier, BigInt(4));

      expect(generatedId.toBase58()).to.deep.equal('7ckT6Y19HnjfqoPFmfL995i4z2HwgZ8UttNmP99LtCBH');
    });
  });
});
