const sinon = require('sinon');
const chai = require('chai');
const chaiAsPromised = require('chai-as-promised');
const Storage = require('../../src/storage');

chai.use(chaiAsPromised);
const { expect } = chai;
const isNode = typeof window === 'undefined';

const fs = isNode ? require('graceful-fs') : {};

let fsData = {};
let localStorageData = {};
const testDataSet = [
  { 'height': 1, 'foo': 'bar' },
  { 'height': 2 },
  { 'height': 3, 'foo': 'bar' },
  { 'height': 4, 'foo': 'bar' },
  { 'height': 5 },
  { 'height': 6, 'foo': 'bar' },
];

let oldStorage;

function createLocalStorageStub() {
  oldStorage = window.localStorage;
  window.__defineGetter__('localStorage', function () {
    return storage;
  });
  const storage = {
    setItem(key, value) {
      localStorageData[key] = value;
    },
    getItem(key) {
      return localStorageData[key];
    },
    removeItem(key) {
      delete localStorageData[key];
    },
    clear() {
      localStorageData = {};
    },
  };
}

function restoreLocalStorageStub() {
  window.__defineGetter__('localStorage', function () {
    return oldStorage;
  });
}

function createFsStub() {
  const writeStub = sinon.stub(fs, 'writeFile');
  writeStub.callsFake((path, data, callback) => {
    fsData[path] = JSON.parse(JSON.stringify(data));
    callback(null);
  });
  const readStub = sinon.stub(fs, 'readFile');
  readStub.callsFake((path, callback) => {
    callback(null, fsData[path]);
  });
}

function restoreFs() {
  fs.writeFile.restore();
  fs.readFile.restore();
}

if (isNode) {
  createFsStub();
} else {
  createLocalStorageStub();
}

describe('Storage', async () => {

  beforeEach(() => {
    if (isNode) {
      fsData = {};
    } else {
      window.localStorage.clear();
    }
  });

  after(() => {
    if (isNode) {
      restoreFs();
    } else {
      restoreLocalStorageStub();
    }
  });

  describe('.getCollection', () => {
    it('Should create collection if it does\'nt exists', async () => {
      const storage = new Storage();

      const collection = await storage.getCollection('blocks');
      expect(Array.isArray(collection.value())).to.be.equal(true);
    });
  });

  describe('.insertOne', () => {
    it('Should add one document to collection', async () => {
      const storage = new Storage();

      let blocks = await storage.findAll('blocks', {});
      expect(blocks.length).to.be.equal(0);

      await storage.insertOne('blocks', testDataSet[0]);
      blocks = await storage.findAll('blocks', {});
      expect(blocks.length).to.be.equal(1);
      expect(blocks[0]).to.be.deep.equal(testDataSet[0]);
    });
    it('Should not add document to collection if document already in collection' +
      'and unique option is specified', async () => {
      const storage = new Storage();

      let blocks = await storage.findAll('blocks', {});
      expect(blocks.length).to.be.equal(0);

      await storage.insertOne('blocks', testDataSet[0], { unique: true });
      blocks = await storage.findAll('blocks', {});
      expect(blocks.length).to.be.equal(1);
      expect(blocks[0]).to.be.deep.equal(testDataSet[0]);

      await storage.insertOne('blocks', testDataSet[0], { unique: true });
      blocks = await storage.findAll('blocks', {});
      expect(blocks.length).to.be.equal(1);
    });
  });

  describe('.insertMany', () => {
    it('Should add many documents to collection', async () => {
      const storage = new Storage();

      let blocks = await storage.findAll('blocks', {});
      expect(blocks.length).to.be.equal(0);

      await storage.insertMany('blocks', testDataSet);
      blocks = await storage.findAll('blocks', {});
      expect(blocks.length).to.be.equal(testDataSet.length);
      blocks.forEach((block, index) => {
        expect(block).to.be.deep.equal(testDataSet[index]);
      });
    });
    it('Should append documents to collection even if they are already included' +
      ' if unique option is not specified', async () => {
      const storage = new Storage();

      let blocks = await storage.findAll('blocks', {});
      expect(blocks.length).to.be.equal(0);

      await storage.insertMany('blocks', testDataSet, { unique: false });
      blocks = await storage.findAll('blocks', {});
      expect(blocks.length).to.be.equal(testDataSet.length);
      blocks.forEach((block, index) => {
        expect(block).to.be.deep.equal(testDataSet[index]);
      });

      await storage.insertMany('blocks', testDataSet.slice(0, 2), { unique: false });
      blocks = await storage.findAll('blocks', {});
      expect(blocks.length).to.be.equal(testDataSet.length + 2);
      blocks.forEach((block, index) => {
        expect(block).to.be.deep.equal(testDataSet[index % testDataSet.length]);
      });
    });
    it(
      'Should not add documents that already in collection if unique option is specified',
      async () => {
        const storage = new Storage();

        let blocks = await storage.findAll('blocks', {});
        expect(blocks.length).to.be.equal(0);

        await storage.insertMany('blocks', testDataSet, { unique: true });
        blocks = await storage.findAll('blocks', {});
        expect(blocks.length).to.be.equal(testDataSet.length);
        blocks.forEach((block, index) => {
          expect(block).to.be.deep.equal(testDataSet[index]);
        });

        await storage.insertMany('blocks', testDataSet.slice(0, 2), { unique: true });
        blocks = await storage.findAll('blocks', {});
        expect(blocks.length).to.be.equal(testDataSet.length);
        blocks.forEach((block, index) => {
          expect(block).to.be.deep.equal(testDataSet[index]);
        });
    });
  });

  describe('.findOne', () => {
    it('Should return one document when comparator object passed', async () => {
      const storage = new Storage();

      await storage.insertMany('blocks', testDataSet, { unique: true });
      let block = await storage.findOne('blocks', {foo: 'bar'});
      expect(block).to.be.deep.equal(testDataSet[0]);
    });
    it('Should return one document when iterator function passed', async () => {
      const storage = new Storage();

      await storage.insertMany('blocks', testDataSet, { unique: true });
      let block = await storage.findOne('blocks', (block) => {
        return block.foo === 'bar'
      });
      expect(block).to.be.deep.equal(testDataSet[0]);
    });
    it('Should return undefined if nothing found', async () => {
      const storage = new Storage();

      await storage.insertMany('blocks', testDataSet, { unique: true });
      let block = await storage.findOne('blocks', {foobar: 'baz'});
      expect(block).to.be.undefined;
    });
  });

  describe('.findAll', () => {
    it('Should return array of documents when comparator object passed', async () => {
      const storage = new Storage();
      const foobarTestDataSet = testDataSet.filter(block => {
        return block.foo === 'bar';
      });

      await storage.insertMany('blocks', testDataSet, { unique: true });
      let blocks = await storage.findAll('blocks', {foo: 'bar'});
      expect(blocks.length).to.be.equal(foobarTestDataSet.length);
      blocks.forEach((block, index) => {
        expect(block).to.be.deep.equal(foobarTestDataSet[index]);
      });
    });
    it('Should return array of documents when iterator function passed', async () => {
      const storage = new Storage();
      const foobarTestDataSet = testDataSet.filter(block => {
        return block.foo === 'bar';
      });

      await storage.insertMany('blocks', testDataSet, { unique: true });
      let blocks = await storage.findAll('blocks', (block) => {
        return block.foo === 'bar';
      });
      expect(blocks.length).to.be.equal(foobarTestDataSet.length);
      blocks.forEach((block, index) => {
        expect(block).to.be.deep.equal(foobarTestDataSet[index]);
      });
    });
    it('Should return empty array if nothing found', async () => {
      const storage = new Storage();

      await storage.insertMany('blocks', testDataSet, { unique: true });
      let blocks = await storage.findAll('blocks', {foobar: 'baz'});
      expect(blocks.length).to.be.equal(0);
    })
  });

  describe('.updateOne', () => {
    it(
      'Should update first document that match search criteria when comparator object passed',
      async () => {
        const storage = new Storage();

        await storage.insertMany('blocks', testDataSet, { unique: true });
        await storage.updateOne('blocks', { foo: 'bar' }, { foo: 'barbaz' });
        let blocks = await storage.findAll('blocks', { foo: 'barbaz' });
        expect(blocks.length).to.be.equal(1);
        expect(blocks[0].height).to.be.equal(testDataSet[0].height);
        expect(blocks[0].foo).to.be.equal('barbaz');
      }
    );
    it(
      'Should update first document that match search criteria when iterator function passed',
      async () => {
        const storage = new Storage();

        await storage.insertMany('blocks', testDataSet, { unique: true });
        await storage.updateOne('blocks', block => block.foo === 'bar', { foo: 'barbaz' });
        let blocks = await storage.findAll('blocks', { foo: 'barbaz' });
        expect(blocks.length).to.be.equal(1);
        expect(blocks[0].height).to.be.equal(testDataSet[0].height);
        expect(blocks[0].foo).to.be.equal('barbaz');
      }
    );
    it(
      'Should not update anything if no matches found',
      async () => {
        const storage = new Storage();

        await storage.insertMany('blocks', testDataSet, { unique: true });
        await storage.updateOne('blocks', { foo: 'barbaz' }, { foo: 'barbazbar' });
        let blocks = await storage.findAll('blocks', { foo: 'barbazbar' });
        expect(blocks.length).to.be.equal(0);
      }
    );
  });

  describe('.updateMany', () => {
    it(
      'Should update all documents that match search criteria when comparator object passed',
      async () => {
        const storage = new Storage();
        const foobarTestDataSet = testDataSet.filter(block => {
          return block.foo === 'bar';
        });

        await storage.insertMany('blocks', testDataSet, { unique: true });
        await storage.updateMany('blocks', { foo: 'bar' }, { foo: 'barbaz' });
        let blocks = await storage.findAll('blocks', { foo: 'barbaz' });
        expect(blocks.length).to.be.equal(4);
        blocks.forEach((block, index) => {
          expect(block.height).to.be.equal(foobarTestDataSet[index].height);
          expect(block.foo).to.be.equal('barbaz');
        });
      }
    );
    it(
      'Should update all documents that match search criteria when iterator function passed',
      async () => {
        const storage = new Storage();
        const foobarTestDataSet = testDataSet.filter(block => {
          return block.foo === 'bar';
        });

        await storage.insertMany('blocks', testDataSet, { unique: true });
        await storage.updateMany('blocks', block => block.foo === 'bar', { foo: 'barbaz' });
        let blocks = await storage.findAll('blocks', { foo: 'barbaz' });
        expect(blocks.length).to.be.equal(4);
        blocks.forEach((block, index) => {
          expect(block.height).to.be.equal(foobarTestDataSet[index].height);
          expect(block.foo).to.be.equal('barbaz');
        });
      }
    );
    it(
      'Should not update anything if no matches found',
      async () => {
        const storage = new Storage();

        await storage.insertMany('blocks', testDataSet, { unique: true });
        await storage.updateMany('blocks', { foo: 'barbaz' }, { foo: 'barbazbar' });
        let blocks = await storage.findAll('blocks', { foo: 'barbazbar' });
        expect(blocks.length).to.be.equal(0);
      }
    );
  });

  describe('.remove()', () => {
    it(
      'Should remove documents that match search criteria when comparator object passed',
      async () => {
        const storage = new Storage();
        const testSet1 = testDataSet.filter(block => block.height !== 3);
        const testSet2 = testDataSet.filter(block => block.foo !== 'bar');

        await storage.insertMany('blocks', testDataSet, { unique: true });

        await storage.remove('blocks', { height: 3 });
        let blocks = await storage.findAll('blocks', {});
        expect(blocks.length).to.be.equal(5);
        blocks.forEach((block, index) => {
          expect(block).to.be.deep.equal(testSet1[index]);
        });
        let deletedBlocks = await storage.findAll('blocks', { height: 3 });
        expect(deletedBlocks.length).to.be.equal(0);

        await storage.remove('blocks', { foo: 'bar' });
        blocks = await storage.findAll('blocks', {});
        expect(blocks.length).to.be.equal(2);
        blocks.forEach((block, index) => {
          expect(block).to.be.deep.equal(testSet2[index]);
        });
        deletedBlocks = await storage.findAll('blocks', { foo: 'bar' });
        expect(deletedBlocks.length).to.be.equal(0);
      }
    );
    it(
      'Should remove documents that match search criteria when iterator function passed',
      async () => {
        const storage = new Storage();
        const testSet1 = testDataSet.filter(block => block.height !== 3);
        const testSet2 = testDataSet.filter(block => block.foo !== 'bar');

        await storage.insertMany('blocks', testDataSet, { unique: true });

        await storage.remove('blocks', block => block.height === 3);
        let blocks = await storage.findAll('blocks', {});
        expect(blocks.length).to.be.equal(5);
        blocks.forEach((block, index) => {
          expect(block).to.be.deep.equal(testSet1[index]);
        });
        let deletedBlocks = await storage.findAll('blocks', { height: 3 });
        expect(deletedBlocks.length).to.be.equal(0);

        await storage.remove('blocks', block => block.foo === 'bar');
        blocks = await storage.findAll('blocks', {});
        expect(blocks.length).to.be.equal(2);
        blocks.forEach((block, index) => {
          expect(block).to.be.deep.equal(testSet2[index]);
        });
        deletedBlocks = await storage.findAll('blocks', { foo: 'bar' });
        expect(deletedBlocks.length).to.be.equal(0);
      }
    );
    it(
      'Should not remove documents if no matches found',
      async () => {
        const storage = new Storage();

        await storage.insertMany('blocks', testDataSet, { unique: true });

        await storage.remove('blocks', { foo: 'barbaz' });
        let blocks = await storage.findAll('blocks', {});
        expect(blocks.length).to.be.equal(testDataSet.length);
        blocks.forEach((block, index) => {
          expect(block).to.be.deep.equal(testDataSet[index]);
        });
      }
    );
  });

});