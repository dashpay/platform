const { mocha: { startMongoDb } } = require('@dashevo/js-evo-services-ctl');

const SVObject = require('../../../../lib/stateView/object/SVObject');
const SVObjectMongoDbRepository = require('../../../../lib/stateView/object/SVObjectMongoDbRepository');

const sanitizer = require('../../../../lib/mongoDb/sanitizer');

const InvalidWhereError = require('../../../../lib/stateView/object/errors/InvalidWhereError');
const InvalidOrderBy = require('../../../../lib/stateView/object/errors/InvalidOrderByError');
const InvalidLimitError = require('../../../../lib/stateView/object/errors/InvalidLimitError');
const InvalidStartAtError = require('../../../../lib/stateView/object/errors/InvalidStartAtError');
const InvalidStartAfterError = require('../../../../lib/stateView/object/errors/InvalidStartAfterError');
const AmbiguousStartError = require('../../../../lib/stateView/object/errors/AmbiguousStartError');

const getSVObjectsFixture = require('../../../../lib/test/fixtures/getSVObjectsFixture');

function sortAndJsonizeSVObjects(svObjects) {
  return svObjects.sort((prev, next) => (
    prev.getDPObject().getId() > next.getDPObject().getId()
  )).map(o => o.toJSON());
}

describe('SVObjectMongoDbRepository', () => {
  let svObjectRepository;
  let svObject;
  let svObjects;
  let mongoDatabase;

  startMongoDb().then((mongoDb) => {
    mongoDatabase = mongoDb.getDb();
  });

  beforeEach(async () => {
    svObjects = getSVObjectsFixture();
    [svObject] = svObjects;

    svObjectRepository = new SVObjectMongoDbRepository(
      mongoDatabase,
      sanitizer,
      svObject.getDPObject().getType(),
    );

    await Promise.all(
      svObjects.map(o => svObjectRepository.store(o)),
    );
  });

  describe('#store', () => {
    it('should store SV Object', async () => {
      const result = await svObjectRepository.find(svObject.getDPObject().getId());

      expect(result).to.be.instanceOf(SVObject);
      expect(result.toJSON()).to.deep.equal(svObject.toJSON());
    });
  });

  describe('#fetch', () => {
    it('should fetch SV Objects', async () => {
      const result = await svObjectRepository.fetch();

      expect(result).to.be.a('array');

      const actualRawSVObjects = sortAndJsonizeSVObjects(result);
      const expectedRawSVObject = sortAndJsonizeSVObjects(svObjects);

      expect(actualRawSVObjects).to.be.deep.equal(expectedRawSVObject);
    });

    it('should not fetch SV Object marked as deleted');

    describe('where', () => {
      it('should fetch SV Objects by where condition', async () => {
        const options = {
          where: { 'dpObject.name': svObject.getDPObject().get('name') },
        };

        const result = await svObjectRepository.fetch(options);

        expect(result).to.be.a('array');

        const [expectedSVObject] = result;

        expect(expectedSVObject.toJSON()).to.be.deep.equal(svObject.toJSON());
      });

      it('should throw InvalidWhereError if where is not an object', async () => {
        const options = {
          where: 'something',
        };

        let error;
        try {
          await svObjectRepository.fetch(options);
        } catch (e) {
          error = e;
        }

        expect(error).to.be.instanceOf(InvalidWhereError);
      });

      it('should throw InvalidWhereError if where is boolean', async () => {
        const options = {
          where: false,
        };

        let error;
        try {
          await svObjectRepository.fetch(options);
        } catch (e) {
          error = e;
        }

        expect(error).to.be.instanceOf(InvalidWhereError);
      });

      it('should return empty array if where conditions do not match', async () => {
        const options = {
          where: { 'dpObject.name': 'Dash enthusiast' },
        };

        const result = await svObjectRepository.fetch(options);

        expect(result).to.be.deep.equal([]);
      });

      it('should throw unknown operator error if where conditions are invalid', async () => {
        const options = {
          where: { 'dpObject.name': { $dirty: true } },
        };

        let error;
        try {
          await svObjectRepository.fetch(options);
        } catch (e) {
          error = e;
        }

        expect(error.message).to.be.equal('unknown operator: $dirty');
      });

      it('should throw unknown operator error if where conditions are invalid', async () => {
        const options = {
          where: { 'dpObject.name': { $dirty: true } },
        };

        let error;
        try {
          await svObjectRepository.fetch(options);
        } catch (e) {
          error = e;
        }

        expect(error.message).to.be.equal('unknown operator: $dirty');
      });
    });

    describe('limit', () => {
      it('should limit return to 1 SV Object if limit', async () => {
        const options = {
          limit: 1,
        };

        const result = await svObjectRepository.fetch(options);

        expect(result).to.be.a('array');
        expect(result).to.have.lengthOf(1);
      });

      it('should throw InvalidLimitError if limit is not a number', async () => {
        const options = {
          limit: 'something',
        };

        let error;
        try {
          await svObjectRepository.fetch(options);
        } catch (e) {
          error = e;
        }

        expect(error).to.be.instanceOf(InvalidLimitError);
      });

      it('should throw InvalidLimitError if limit is boolean', async () => {
        const options = {
          limit: false,
        };

        let error;
        try {
          await svObjectRepository.fetch(options);
        } catch (e) {
          error = e;
        }

        expect(error).to.be.instanceOf(InvalidLimitError);
      });
    });

    describe('orderBy', () => {
      it('should order desc', async () => {
        svObjects.forEach((o, i) => o.getDPObject().set('age', i + 1));

        await Promise.all(
          svObjects.map(o => svObjectRepository.store(o)),
        );

        const options = {
          orderBy: {
            'dpObject.age': -1,
          },
        };

        const result = await svObjectRepository.fetch(options);

        expect(result).to.be.a('array');

        const actualRawSVObjects = result.map(o => o.toJSON());
        const expectedRawSVObjects = svObjects.reverse().map(o => o.toJSON());

        expect(actualRawSVObjects).to.be.deep.equal(expectedRawSVObjects);
      });

      it('should order asc', async () => {
        svObjects.reverse().forEach((o, i) => o.getDPObject().set('age', i + 1));

        await Promise.all(
          svObjects.map(o => svObjectRepository.store(o)),
        );

        const options = {
          orderBy: {
            'dpObject.age': 1,
          },
        };

        const result = await svObjectRepository.fetch(options);

        expect(result).to.be.a('array');

        const actualRawSVObjects = result.map(o => o.toJSON());
        const expectedRawSVObjects = svObjects.map(o => o.toJSON());

        expect(actualRawSVObjects).to.be.deep.equal(expectedRawSVObjects);
      });

      it('should throw InvalidOrderBy if orderBy is not an object', async () => {
        const options = {
          orderBy: 'something',
        };

        let error;
        try {
          await svObjectRepository.fetch(options);
        } catch (e) {
          error = e;
        }

        expect(error).to.be.instanceOf(InvalidOrderBy);
      });

      it('should throw InvalidOrderBy if orderBy is boolean', async () => {
        const options = {
          orderBy: false,
        };

        let error;
        try {
          await svObjectRepository.fetch(options);
        } catch (e) {
          error = e;
        }

        expect(error).to.be.instanceOf(InvalidOrderBy);
      });
    });

    describe('start', () => {
      it('should start at 1 object', async () => {
        svObjects.forEach((o, i) => o.getDPObject().set('age', i + 1));

        await Promise.all(
          svObjects.map(o => svObjectRepository.store(o)),
        );

        const options = {
          orderBy: {
            'dpObject.age': 1,
          },
          startAt: 2,
        };

        const result = await svObjectRepository.fetch(options);

        expect(result).to.be.a('array');

        const actualRawSVObjects = result.map(o => o.toJSON());
        const expectedRawSVObjects = svObjects.splice(1).map(o => o.toJSON());

        expect(actualRawSVObjects).to.be.deep.equal(expectedRawSVObjects);
      });

      it('should throw InvalidStartAtError if startAt is not a number', async () => {
        const options = {
          startAt: 'something',
        };

        let error;
        try {
          await svObjectRepository.fetch(options);
        } catch (e) {
          error = e;
        }

        expect(error).to.be.instanceOf(InvalidStartAtError);
      });

      it('should throw InvalidStartAtError if startAt is boolean', async () => {
        const options = {
          startAt: 'something',
        };

        let error;
        try {
          await svObjectRepository.fetch(options);
        } catch (e) {
          error = e;
        }

        expect(error).to.be.instanceOf(InvalidStartAtError);
      });

      it('should start after 1 object', async () => {
        svObjects.forEach((o, i) => o.getDPObject().set('age', i + 1));

        await Promise.all(
          svObjects.map(o => svObjectRepository.store(o)),
        );

        const options = {
          orderBy: {
            'dpObject.age': 1,
          },
          startAfter: 1,
        };

        const result = await svObjectRepository.fetch(options);

        expect(result).to.be.a('array');

        const actualRawSVObjects = result.map(o => o.toJSON());
        const expectedRawSVObjects = svObjects.splice(1).map(o => o.toJSON());

        expect(actualRawSVObjects).to.be.deep.equal(expectedRawSVObjects);
      });

      it('should throw InvalidStartAfterError if startAfter is not a number', async () => {
        const options = {
          startAfter: 'something',
        };

        let error;
        try {
          await svObjectRepository.fetch(options);
        } catch (e) {
          error = e;
        }

        expect(error).to.be.instanceOf(InvalidStartAfterError);
      });

      it('should throw InvalidStartAfterError if startAfter is boolean', async () => {
        const options = {
          startAfter: false,
        };

        let error;
        try {
          await svObjectRepository.fetch(options);
        } catch (e) {
          error = e;
        }

        expect(error).to.be.instanceOf(InvalidStartAfterError);
      });

      it('should throw AmbiguousStartError if the both startAt and startAfter are present', async () => {
        let error;

        try {
          await svObjectRepository.fetch({ startAt: 1, startAfter: 2 });
        } catch (e) {
          error = e;
        }

        expect(error).to.be.instanceOf(AmbiguousStartError);
      });
    });
  });

  describe('#findAllBySTHash', () => {
    it('should find all SV Objects by stHash', async () => {
      const stHash = svObject.getReference().getSTHash();

      const result = await svObjectRepository.findAllBySTHash(stHash);

      expect(result).to.be.a('array');

      const [expectedSVObject] = result;

      expect(expectedSVObject.toJSON()).to.be.deep.equal(svObject.toJSON());
    });
  });

  describe('#delete', () => {
    it('should delete SV Object', async () => {
      await svObjectRepository.delete(svObject);

      const result = await svObjectRepository.find(svObject.getDPObject().getId());

      expect(result).to.be.null();
    });
  });

  describe('#find', () => {
    it('should find SV Object by ID');

    it('should find SV Object marked as deleted by ID');

    it('should return null if SV object not found', async () => {
      const object = await svObjectRepository.find('unknown');

      expect(object).to.be.null();
    });
  });
});
