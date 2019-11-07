const findNotIndexedFields = require('../../../../../lib/stateView/document/query/findNotIndexedFields');

describe('findNotIndexedFields', () => {
  let indexedFields;

  beforeEach(() => {
    indexedFields = [
      [{ $userId: 'asc' }, { firstName: 'desc' }],
      [{ $userId: 'asc' }, { lastName: 'desc' }, { secondName: 'asc' }],
      [{ $id: 'asc' }],
      [{ $id: 'desc' }],
      [{ 'arrayWithObjects.item': 'desc' }],
      [{ 'arrayWithObjects.flag': 'desc' }],
      [{ address: 'asc' }, { 'arrayWithObjects.flag': 'desc' }],
      [{ 'arrayWithObjects.flag': 'desc' }, { street: 'asc' }],
      [{ 'arrayWithObjects.country': 'desc' }, { 'arrayWithObjects.language': 'asc' }],
    ];
  });

  it('should pass system $id field', () => {
    const condition = [['$id', '==', 123]];
    const result = findNotIndexedFields(indexedFields, condition);

    expect(result).to.be.an('array');
    expect(result).to.be.empty();
  });

  it('should fail with condition by second field of compound index', () => {
    const condition = [['firstName', '==', 'name']];
    const result = findNotIndexedFields(indexedFields, condition);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(1);
    expect(result[0]).to.equal('firstName');
  });

  it('should return an error for one field', () => {
    const condition = [['secondName', '==', 'name']];
    const result = findNotIndexedFields(indexedFields, condition);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(1);
    expect(result[0]).to.equal('secondName');
  });

  it('should return an error for three fields', () => {
    const condition = [['secondName', '==', 'name'], ['city', '==', 'NY'], ['country', '==', 'USA']];
    const result = findNotIndexedFields(indexedFields, condition);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(3);
    expect(result).to.have.deep.members(['secondName', 'city', 'country']);
  });

  it('should check empty condition', () => {
    const condition = [];
    const result = findNotIndexedFields(indexedFields, condition);

    expect(result).to.be.an('array');
    expect(result).to.be.empty();
  });

  it('should check empty document indices', () => {
    delete indexedFields.indices;
    const condition = [['secondName', '==', 'name']];

    const result = findNotIndexedFields(indexedFields, condition);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(1);
    expect(result[0]).to.equal('secondName');
  });

  it('should check fields by nested conditions', () => {
    const condition = [
      ['$userId', '==', 'Cutie'],
      ['arrayWithObjects', 'elementMatch', [
        ['item', '==', 1],
        ['flag', '==', true],
      ]],
    ];

    const result = findNotIndexedFields(indexedFields, condition);

    expect(result).to.be.an('array');
    expect(result).to.be.empty();
  });

  it('should fail with nested conditions', () => {
    const condition = [
      ['$userId', '==', 123],
      ['arrayWithObjects', 'elementMatch', [
        ['item', '==', 1],
        ['anotherFlag', '==', true],
      ]],
    ];

    const result = findNotIndexedFields(indexedFields, condition);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(1);
    expect(result[0]).to.equal('arrayWithObjects.anotherFlag');
  });

  it('should pass query by compound index', () => {
    const condition = [['firstName', '==', 'name'], ['$userId', '==', 123]];
    const result = findNotIndexedFields(indexedFields, condition);

    expect(result).to.be.an('array');
    expect(result).to.be.empty();
  });

  it('should fail with query by third field of compound index', () => {
    const condition = [['lastName', '==', 'name'], ['secondName', '==', 'myName']];
    const result = findNotIndexedFields(indexedFields, condition);

    expect(result).to.have.lengthOf(2);
    expect(result).to.have.members(['lastName', 'secondName']);
  });

  it('should check fields by nested conditions in compound index when nested condition is second index', () => {
    const condition = [
      ['address', '==', 'myAddress'],
      ['arrayWithObjects', 'elementMatch', [
        ['item', '==', 1],
        ['flag', '==', true],
      ]],
    ];

    const result = findNotIndexedFields(indexedFields, condition);

    expect(result).to.be.an('array');
    expect(result).to.be.empty();
  });

  it('should check fields by nested conditions in compound index when nested condition is first index', () => {
    const condition = [
      ['street', '==', 'myStreet'],
      ['arrayWithObjects', 'elementMatch', [
        ['item', '==', 1],
        ['flag', '==', true],
      ]],
    ];

    const result = findNotIndexedFields(indexedFields, condition);

    expect(result).to.be.an('array');
    expect(result).to.be.empty();
  });

  it('should check fields by nested conditions in compound index when all indexes are nested', () => {
    const condition = [
      ['arrayWithObjects', 'elementMatch', [
        ['country', '==', 'USA'],
        ['language', '==', 'english'],
      ]],
    ];

    const result = findNotIndexedFields(indexedFields, condition);

    expect(result).to.be.an('array');
    expect(result).to.be.empty();
  });

  it('should fail when we miss first part of nested field of nested compound index', () => {
    const condition = [
      ['arrayWithObjects', 'elementMatch', [
        ['language', '==', 'english'],
        ['flag', '==', true],
      ]],
    ];

    const result = findNotIndexedFields(indexedFields, condition);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(1);
    expect(result).to.have.members(['arrayWithObjects.language']);
  });
});
